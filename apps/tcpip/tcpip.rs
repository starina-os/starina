use ftl_api::channel::ChannelSender;
use ftl_api::collections::HashMap;
use ftl_api::prelude::*;
use ftl_api::types::error::FtlError;
use smoltcp::iface::Config;
use smoltcp::iface::Interface;
use smoltcp::iface::SocketHandle;
use smoltcp::iface::SocketSet;
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpListenEndpoint;

use crate::device::NetDevice;

fn now() -> Instant {
    // FIXME:
    Instant::from_millis(0)
}

#[derive(Debug, PartialEq, Eq)]
enum SocketState {
    Listening { listen_endpoint: IpListenEndpoint },
    Established,
    Closed,
}

struct Socket {
    ch: ChannelSender,
    smol_handle: SocketHandle,
    state: SocketState,
}

#[derive(Debug)]
pub enum SocketEvent<'a> {
    NewConnection {
        ch: &'a mut ChannelSender,
        smol_handle: SocketHandle,
    },
    Data {
        ch: &'a ChannelSender,
        data: &'a [u8],
    },
    Close {
        ch: &'a ChannelSender,
    },
}

pub struct TcpIp<'a> {
    recv_buf: Vec<u8>,
    smol_sockets: SocketSet<'a>,
    device: NetDevice,
    iface: Interface,
    sockets: HashMap<SocketHandle, Socket>,
}

impl<'a> TcpIp<'a> {
    pub fn new(mut device: NetDevice, hwaddr: HardwareAddress) -> TcpIp<'a> {
        let config = Config::new(hwaddr.into());
        let mut iface = Interface::new(config, &mut device, now());
        let smol_sockets = SocketSet::new(Vec::with_capacity(16));

        // FIXME:
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs
                .push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24))
                .unwrap();
        });

        TcpIp {
            device,
            iface,
            smol_sockets,
            sockets: HashMap::new(),
            recv_buf: vec![0; 1514],
        }
    }

    pub fn poll<F>(&mut self, mut callback: F)
    where
        F: FnMut(SocketEvent<'_>),
    {
        loop {
            let progress = self
                .iface
                .poll(now(), &mut self.device, &mut self.smol_sockets);

            if !progress {
                // No changes in smoltcp.
                break;
            }

            let mut needs_listen = Vec::new();
            for (handle, sock) in self.sockets.iter_mut() {
                let smol_sock = self.smol_sockets.get_mut::<tcp::Socket>(sock.smol_handle);
                match (&mut sock.state, smol_sock.state()) {
                    (
                        SocketState::Listening { .. },
                        tcp::State::Listen | tcp::State::SynReceived,
                    ) => {
                        // Do nothing.
                    }
                    (SocketState::Listening { listen_endpoint }, tcp::State::Established) => {
                        // Create a new listening socket as this one is now
                        // established.
                        //
                        // Note: This should be done before calling the callback
                        //       becaseu it may overwrite `ch` and drop the
                        //       previous one.
                        needs_listen.push((*listen_endpoint, sock.ch.clone()));

                        // The listening socket has transitioned to established.
                        callback(SocketEvent::NewConnection {
                            ch: &mut sock.ch,
                            smol_handle: *handle,
                        });

                        sock.state = SocketState::Established;
                    }
                    (SocketState::Listening { .. }, _) => {
                        // Inactive, closed, or unknown state. Close the socket.
                        callback(SocketEvent::Close { ch: &sock.ch });
                        sock.state = SocketState::Closed;
                    }
                    (SocketState::Established, _) if smol_sock.can_recv() => {
                        // The establish connection with some received data.
                        loop {
                            let len = smol_sock.recv_slice(self.recv_buf.as_mut_slice()).unwrap();
                            if len == 0 {
                                break;
                            }

                            callback(SocketEvent::Data {
                                ch: &sock.ch,
                                data: &self.recv_buf[..len],
                            });
                        }
                    }
                    (SocketState::Established, tcp::State::Established) => {
                        // Do nothing.
                    }
                    (SocketState::Established, _) => {
                        // Unknown state. Close the connection.
                        callback(SocketEvent::Close { ch: &sock.ch });
                        sock.state = SocketState::Closed;
                    }
                    (SocketState::Closed, _) => {
                        unreachable!();
                    }
                }
            }

            // Remove closed sockets from self.sockets and smoltcp's socket set.
            self.sockets.retain(|handle, sock| {
                if sock.state == SocketState::Closed {
                    debug_warn!("closing socket {:?}", handle);
                    self.smol_sockets.remove(*handle);
                    false
                } else {
                    true
                }
            });

            for (listen_endpoint, ch) in needs_listen {
                self.replenish_listen_sock(listen_endpoint, ch);
            }
        }
    }

    fn replenish_listen_sock(&mut self, listen_endpoint: IpListenEndpoint, ch: ChannelSender) {
        let rx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let tx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let mut sock = tcp::Socket::new(rx_buf, tx_buf);
        sock.listen(listen_endpoint).unwrap();

        let handle = self.smol_sockets.add(sock);
        self.sockets.insert(
            handle,
            Socket {
                ch,
                smol_handle: handle,
                state: SocketState::Listening { listen_endpoint },
            },
        );
    }

    pub fn tcp_listen(
        &mut self,
        listen_endpoint: IpListenEndpoint,
        ch: ChannelSender,
    ) -> Result<(), FtlError> {
        self.replenish_listen_sock(listen_endpoint, ch);
        Ok(())
    }

    pub fn tcp_send(&mut self, handle: SocketHandle, data: &[u8]) -> Result<(), FtlError> {
        let socket = self
            .sockets
            .get_mut(&handle)
            .ok_or(FtlError::HandleNotFound)?;

        if !matches!(socket.state, SocketState::Established { .. }) {
            return Err(FtlError::InvalidState);
        }

        // Write the data to the TCP buffer.
        if self
            .smol_sockets
            .get_mut::<tcp::Socket>(socket.smol_handle)
            .send_slice(data)
            .is_err()
        {
            return Err(FtlError::InvalidState);
        }

        Ok(())
    }

    pub fn receive_packet(&mut self, pkt: &[u8]) {
        self.device.receive_pkt(pkt);
    }
}
