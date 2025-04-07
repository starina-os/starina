use core::net::Ipv4Addr;

use smoltcp::iface::Config;
use smoltcp::iface::Interface;
use smoltcp::iface::PollResult;
use smoltcp::iface::SocketHandle;
use smoltcp::iface::SocketSet;
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpEndpoint;
use smoltcp::wire::IpListenEndpoint;
use starina::channel::ChannelSender;
use starina::collections::HashMap;
use starina::error::ErrorCode;
use starina::prelude::*;

use crate::device::NetDevice;

fn now() -> Instant {
    // FIXME:
    Instant::from_millis(0)
}

#[derive(Debug, PartialEq, Eq)]
enum SocketState {
    Connecting { remote_endpoint: IpEndpoint },
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
    /// Our socket states. The key is the smoltcp socket handle.
    sockets: HashMap<SocketHandle, Socket>,
    /// The smoltcp socket states.
    smol_sockets: SocketSet<'a>,
    device: NetDevice,
    recv_buf: Vec<u8>,
    iface: Interface,
    next_ephemeral_port: u16,
}

impl<'a> TcpIp<'a> {
    pub fn new(
        mut device: NetDevice,
        our_ip: IpCidr,
        gw_ip: Ipv4Addr,
        hwaddr: HardwareAddress,
    ) -> TcpIp<'a> {
        let config = Config::new(hwaddr);
        let mut iface = Interface::new(config, &mut device, now());
        let smol_sockets = SocketSet::new(Vec::with_capacity(16));

        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs.push(our_ip).unwrap();
        });

        iface.routes_mut().add_default_ipv4_route(gw_ip).unwrap();

        TcpIp {
            device,
            iface,
            smol_sockets,
            sockets: HashMap::new(),
            recv_buf: vec![0; 1514],
            next_ephemeral_port: 49152,
        }
    }

    /// Makes progress in smoltcp - receive RX packets, update socket states,
    /// and transmit TX packets.
    ///
    /// As it detects changes in socket states, it calls the `callback` so that
    /// we can do message passing in the main loop.
    pub fn poll<C, F>(&mut self, ctx: &C, mut callback: F)
    where
        F: FnMut(&C, SocketEvent<'_>),
    {
        loop {
            let result = self
                .iface
                .poll(now(), &mut self.device, &mut self.smol_sockets);

            match result {
                PollResult::SocketStateChanged => {
                    // Continue processing sockets.
                }
                PollResult::None => {
                    // No changes in smoltcp.
                    break;
                }
            }

            let mut needs_listen = Vec::new();
            for (handle, sock) in self.sockets.iter_mut() {
                let smol_sock = self.smol_sockets.get_mut::<tcp::Socket>(sock.smol_handle);
                match (&mut sock.state, smol_sock.state()) {
                    (SocketState::Connecting { .. }, tcp::State::SynSent) => {
                        trace!("connecting socket {:?}", handle);
                    }
                    (SocketState::Connecting { .. }, _state) => {
                        todo!();
                    }
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
                        callback(
                            ctx,
                            SocketEvent::NewConnection {
                                ch: &mut sock.ch,
                                smol_handle: *handle,
                            },
                        );

                        sock.state = SocketState::Established;
                    }
                    (SocketState::Listening { .. }, _) => {
                        // Inactive, closed, or unknown state. Close the socket.
                        callback(ctx, SocketEvent::Close { ch: &sock.ch });
                        sock.state = SocketState::Closed;
                    }
                    (SocketState::Established, _) if smol_sock.can_recv() => {
                        // The establish connection with some received data.
                        loop {
                            let len = smol_sock.recv_slice(self.recv_buf.as_mut_slice()).unwrap();
                            if len == 0 {
                                break;
                            }

                            callback(
                                ctx,
                                SocketEvent::Data {
                                    ch: &sock.ch,
                                    data: &self.recv_buf[..len],
                                },
                            );
                        }
                    }
                    (SocketState::Established, tcp::State::Established) => {
                        // Do nothing.
                    }
                    (SocketState::Established, _) => {
                        // Unknown state. Close the connection.
                        callback(ctx, SocketEvent::Close { ch: &sock.ch });
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

    fn get_ephemeral_port(&mut self) -> Result<u16, ErrorCode> {
        let port = self.next_ephemeral_port;
        self.next_ephemeral_port += 1;
        Ok(port)
    }

    pub fn tcp_connect(
        &mut self,
        remote_endpoint: IpEndpoint,
        ch: ChannelSender,
    ) -> Result<(), ErrorCode> {
        let rx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let tx_buf = tcp::SocketBuffer::new(vec![0; 8192]);
        let mut sock = tcp::Socket::new(rx_buf, tx_buf);
        let our_port = self.get_ephemeral_port()?;
        sock.connect(self.iface.context(), remote_endpoint, our_port)
            .unwrap(); // FIXME:

        let handle = self.smol_sockets.add(sock);
        self.sockets.insert(
            handle,
            Socket {
                ch,
                smol_handle: handle,
                state: SocketState::Connecting { remote_endpoint },
            },
        );

        Ok(())
    }

    pub fn tcp_listen(
        &mut self,
        listen_endpoint: IpListenEndpoint,
        ch: ChannelSender,
    ) -> Result<(), ErrorCode> {
        self.replenish_listen_sock(listen_endpoint, ch);
        Ok(())
    }

    pub fn tcp_send(&mut self, handle: SocketHandle, data: &[u8]) -> Result<(), ErrorCode> {
        let socket = self.sockets.get_mut(&handle).ok_or(ErrorCode::NotFound)?;

        if !matches!(socket.state, SocketState::Established { .. }) {
            return Err(ErrorCode::InvalidState);
        }

        // Write the data to the TCP buffer.
        if self
            .smol_sockets
            .get_mut::<tcp::Socket>(socket.smol_handle)
            .send_slice(data)
            .is_err()
        {
            return Err(ErrorCode::InvalidState);
        }

        Ok(())
    }

    pub fn receive_packet(&mut self, pkt: &[u8]) {
        self.device.receive_pkt(pkt);
    }
}
