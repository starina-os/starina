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
use smoltcp::wire::IpListenEndpoint;
use starina::channel::ChannelSender;
use starina::collections::HashMap;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::Handleable;
use starina::prelude::*;
use starina::timer;

use crate::device::NetDevice;

fn now() -> Instant {
    let monotonic_time = timer::now();
    Instant::from_millis(monotonic_time.as_millis() as i64)
}

#[derive(Debug, PartialEq, Eq)]
enum SocketState {
    Listening { listen_endpoint: IpListenEndpoint },
    Established,
    Closing, // Graceful shutdown in progress
    Closed,
}

struct Socket {
    ch: ChannelSender,
    smol_handle: SocketHandle,
    state: SocketState,
    backpressured: bool,
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
    Closed {
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
        }
    }

    pub fn close_socket(&mut self, handle: SocketHandle) -> Result<(), ErrorCode> {
        let socket = self.sockets.get_mut(&handle).ok_or(ErrorCode::NotFound)?;

        // Only initiate graceful shutdown if socket is established
        if socket.state == SocketState::Established {
            let smol_sock = self.smol_sockets.get_mut::<tcp::Socket>(socket.smol_handle);
            smol_sock.close();
            socket.state = SocketState::Closing;
        } else {
            socket.state = SocketState::Closed;
        }

        Ok(())
    }

    /// Makes progress in smoltcp - receive RX packets, update socket states,
    /// and transmit TX packets.
    ///
    /// As it detects changes in socket states, it calls the `callback` so that
    /// we can do message passing in the main loop.
    pub fn poll<F>(&mut self, mut callback: F)
    where
        F: FnMut(SocketEvent<'_>),
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
                    (SocketState::Listening { .. }, tcp::State::Listen) => {
                        // Do nothing.
                    }
                    (SocketState::Listening { listen_endpoint }, tcp::State::SynReceived) => {
                        // SYN received - immediately create replacement listener to handle concurrent connections.
                        needs_listen.push((*listen_endpoint, sock.ch.clone()));
                    }
                    (SocketState::Listening { .. }, tcp::State::Established) => {
                        // The listening socket has transitioned to established.
                        callback(SocketEvent::NewConnection {
                            ch: &mut sock.ch,
                            smol_handle: *handle,
                        });

                        sock.state = SocketState::Established;
                    }
                    (SocketState::Listening { .. }, tcp::State::Closed) => {
                        // Socket transitioned to closed state. Close the socket.
                        callback(SocketEvent::Closed { ch: &sock.ch });
                        sock.state = SocketState::Closed;
                    }
                    (SocketState::Listening { .. }, smol_state) => {
                        unreachable!("unexpected state in Listening: {:?}", smol_state);
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
                    (SocketState::Established, tcp::State::CloseWait) => {
                        // Remote peer closed their side, transition to closing
                        trace!("socket {:?} is closed by remote peer", handle);
                        sock.state = SocketState::Closing;
                        callback(SocketEvent::Closed { ch: &sock.ch });
                    }
                    (SocketState::Established, smol_state) => {
                        unreachable!("unexpected state in Established: {:?}", smol_state);
                    }
                    (SocketState::Closing, tcp::State::TimeWait | tcp::State::Closed) => {
                        // Graceful shutdown complete, remove the socket
                        sock.state = SocketState::Closed;
                        callback(SocketEvent::Closed { ch: &sock.ch });
                    }
                    (SocketState::Closing, tcp::State::FinWait1 | tcp::State::FinWait2) => {
                        // Still in graceful shutdown, keep socket alive
                        // Continue sending any remaining data
                        trace!("socket {:?} is in FinWait", handle);
                    }
                    (SocketState::Closing, tcp::State::CloseWait) => {
                        trace!("socket {:?} is in CloseWait", handle);
                    }
                    (SocketState::Closing, smol_state) => {
                        unreachable!("unexpected state in Closing: {:?}", smol_state);
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
                backpressured: false,
            },
        );
    }

    pub fn tcp_listen(
        &mut self,
        listen_endpoint: IpListenEndpoint,
        ch: ChannelSender,
    ) -> Result<(), ErrorCode> {
        self.replenish_listen_sock(listen_endpoint, ch);
        Ok(())
    }

    pub fn tcp_sendable_len(&self, handle: SocketHandle) -> Result<usize, ErrorCode> {
        let socket = self.sockets.get(&handle).ok_or(ErrorCode::NotFound)?;
        let smol_sock = self.smol_sockets.get::<tcp::Socket>(socket.smol_handle);
        Ok(smol_sock.send_capacity() - smol_sock.send_queue())
    }

    pub fn tcp_send(&mut self, handle: SocketHandle, data: &[u8]) -> Result<usize, ErrorCode> {
        let socket = self.sockets.get_mut(&handle).ok_or(ErrorCode::NotFound)?;

        if !matches!(socket.state, SocketState::Established) {
            return Err(ErrorCode::InvalidState);
        }

        let smol_sock = self.smol_sockets.get_mut::<tcp::Socket>(socket.smol_handle);
        let written_len = smol_sock
            .send_slice(data)
            .map_err(|_| ErrorCode::InvalidState)?;

        Ok(written_len)
    }

    pub fn receive_packet(&mut self, pkt: &[u8]) {
        self.device.receive_pkt(pkt);
    }

    pub fn mark_as_backpressured(&mut self, handle: SocketHandle) {
        if let Some(socket) = self.sockets.get_mut(&handle) {
            socket.backpressured = true;
        }
    }

    pub fn get_writeable_sockets(&mut self) -> Vec<(HandleId, SocketHandle)> {
        let mut sockets = Vec::new();
        for (handle, socket) in self.sockets.iter_mut() {
            if !socket.backpressured {
                continue;
            }

            if !matches!(socket.state, SocketState::Established) {
                continue;
            }

            let smol_sock = self.smol_sockets.get::<tcp::Socket>(socket.smol_handle);
            let writeable_len = smol_sock.send_capacity() - smol_sock.send_queue();
            if writeable_len > 0 {
                socket.backpressured = false;
                sockets.push((socket.ch.handle_id(), *handle));
            }
        }

        sockets
    }
}
