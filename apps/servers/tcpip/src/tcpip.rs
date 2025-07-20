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
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::channel::ChannelSender;
use starina::channel::RecvError;
use starina::collections::HashMap;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::MESSAGE_DATA_LEN_MAX;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::timer;

use crate::State;
use crate::device::NetDevice;

fn parse_addr(addr: &str) -> Option<(core::net::Ipv4Addr, u16)> {
    let mut parts = addr.split(':');
    let ip = parts.next()?.parse().ok()?;
    let port = parts.next()?.parse().ok()?;
    Some((ip, port))
}

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

pub struct TcpIp {
    /// Our socket states. The key is the smoltcp socket handle.
    sockets: HashMap<SocketHandle, Socket>,
    /// The smoltcp socket states.
    smol_sockets: SocketSet<'static>,
    device: NetDevice,
    recv_buf: Vec<u8>,
    iface: Interface,
}

fn process_tcp_state(
    handle: SocketHandle,
    sock: &mut Socket,
    smol_sock: &mut tcp::Socket,
    needs_listen: &mut Vec<(IpListenEndpoint, ChannelSender)>,
    recv_buf: &mut [u8],
    poll: &Poll<State>,
) {
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
            let (our_ch, their_ch) = Channel::new().unwrap();
            let (our_tx, our_rx) = our_ch.split();
            poll.add(
                our_rx.handle_id(),
                State::Data {
                    smol_handle: handle,
                    ch: our_rx,
                },
                Readiness::READABLE | Readiness::CLOSED,
            )
            .expect("failed to get channel sender");

            sock.ch.send(Message::Connect { ch: their_ch }).unwrap();
            sock.ch = our_tx;
            sock.state = SocketState::Established;
        }
        (SocketState::Listening { .. }, tcp::State::Closed) => {
            // Socket transitioned to closed state. Close the socket.
            debug_warn!("socket fully closed, cleaning up channel");
            if let Err(err) = poll.remove(sock.ch.handle_id()) {
                debug_warn!(
                    "failed to remove channel from poll (already removed?): {:?}",
                    err
                );
            }
            sock.state = SocketState::Closed;
        }
        (SocketState::Listening { .. }, smol_state) => {
            unreachable!("unexpected state in Listening: {:?}", smol_state);
        }
        (SocketState::Established, _) if smol_sock.can_recv() => {
            // The establish connection with some received data.
            loop {
                let len = smol_sock.recv_slice(recv_buf).unwrap();
                if len == 0 {
                    break;
                }

                sock.ch
                    .send(Message::Data {
                        data: &recv_buf[..len],
                    })
                    .unwrap();
            }
        }
        (SocketState::Established, tcp::State::Established) => {
            // Do nothing.
        }
        (SocketState::Established, tcp::State::CloseWait) => {
            // Remote peer closed their side, transition to closing
            trace!("socket {:?} is closed by remote peer", handle);
            sock.state = SocketState::Closing;
            debug_warn!("socket fully closed, cleaning up channel");
            if let Err(err) = poll.remove(sock.ch.handle_id()) {
                debug_warn!(
                    "failed to remove channel from poll (already removed?): {:?}",
                    err
                );
            }
        }
        (SocketState::Established, smol_state) => {
            unreachable!("unexpected state in Established: {:?}", smol_state);
        }
        (SocketState::Closing, tcp::State::TimeWait | tcp::State::Closed) => {
            // Graceful shutdown complete, remove the socket
            sock.state = SocketState::Closed;
            debug_warn!("socket fully closed, cleaning up channel");
            if let Err(err) = poll.remove(sock.ch.handle_id()) {
                debug_warn!(
                    "failed to remove channel from poll (already removed?): {:?}",
                    err
                );
            }
        }
        (SocketState::Closing, tcp::State::FinWait1 | tcp::State::FinWait2) => {
            // Still in graceful shutdown, keep socket alive
            // Continue sending any remaining data
            trace!("socket {:?} is in FinWait", handle);
        }
        (SocketState::Closing, tcp::State::CloseWait) => {
            trace!("socket {:?} is in CloseWait", handle);
        }
        (SocketState::Closing, tcp::State::Closing) => {
            // Waiting for the remote peer to acknowledge our FIN.
        }
        (SocketState::Closing, smol_state) => {
            unreachable!("unexpected state in Closing: {:?}", smol_state);
        }
        (SocketState::Closed, _) => {
            unreachable!();
        }
    }
}

impl TcpIp {
    pub fn new(
        mut device: NetDevice,
        our_ip: IpCidr,
        gw_ip: Ipv4Addr,
        hwaddr: HardwareAddress,
    ) -> TcpIp {
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
    pub fn poll(&mut self, poll: &Poll<State>) {
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
                process_tcp_state(
                    *handle,
                    sock,
                    smol_sock,
                    &mut needs_listen,
                    &mut self.recv_buf,
                    poll,
                );
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
                self.create_listen_sock(listen_endpoint, ch);
            }
        }

        for (ch_handle_id, smol_handle) in self.get_writeable_sockets() {
            trace!(
                "write-backpressued socket is now writeable: {:?}",
                smol_handle
            );
            poll.listen(ch_handle_id, Readiness::READABLE | Readiness::CLOSED)
                .unwrap();
        }
    }

    fn create_listen_sock(&mut self, listen_endpoint: IpListenEndpoint, ch: ChannelSender) {
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
        self.create_listen_sock(listen_endpoint, ch);
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

    pub fn handle_startup_channel(
        &mut self,
        poll: &Poll<State>,
        ch: &Channel,
        msgbuffer: &mut MessageBuffer,
    ) {
        match ch.recv(msgbuffer) {
            Ok(Message::Connect { ch }) => {
                self.handle_startup_connect(poll, ch);
            }
            Ok(msg) => {
                debug_warn!("unexpected message on startup channel: {:?}", msg);
            }
            Err(RecvError::Parse(msginfo)) => {
                debug_warn!(
                    "malformed message on startup channel: {}",
                    msginfo.kind()
                );
            }
            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
            Err(RecvError::Syscall(err)) => {
                debug_warn!("recv error on startup channel: {:?}", err);
            }
        }
    }

    pub fn handle_startup_connect(&mut self, poll: &Poll<State>, handle: Channel) {
        poll.add(
            handle.handle_id(),
            State::Control(handle),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();
    }

    pub fn handle_control_channel(
        &mut self,
        poll: &Poll<State>,
        ch: &Channel,
        msgbuffer: &mut MessageBuffer,
    ) {
        match ch.recv(msgbuffer) {
            Ok(Message::Open { call_id, uri }) => {
                self.handle_control_open(poll, ch, call_id, uri);
            }
            Ok(msg) => {
                debug_warn!("unexpected message on control channel: {:?}", msg);
            }
            Err(RecvError::Parse(msginfo)) => {
                debug_warn!(
                    "malformed message on control channel: {}",
                    msginfo.kind()
                );
            }
            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
            Err(RecvError::Syscall(err)) => {
                debug_warn!("recv error on control channel: {:?}", err);
            }
        }
    }

    pub fn handle_control_close(&mut self, poll: &Poll<State>, ch: &Channel) {
        debug_warn!("control channel closed");
        poll.remove(ch.handle_id()).unwrap();
    }

    pub fn handle_control_open(
        &mut self,
        poll: &Poll<State>,
        ch: &Channel,
        call_id: CallId,
        uri: &[u8],
    ) {
        let uri = core::str::from_utf8(uri).unwrap();
        info!("got open message: {}", uri);
        let Some(("tcp-listen", rest)) = uri.split_once(':') else {
            ch.send(Message::Abort {
                call_id,
                reason: ErrorCode::InvalidUri,
            })
            .unwrap();
            return;
        };

        let Some((ip, port)) = parse_addr(rest) else {
            debug_warn!("invalid tcp-listen message: {}", uri);
            ch.send(Message::Abort {
                call_id,
                reason: ErrorCode::InvalidUri,
            })
            .unwrap();
            return;
        };

        let listen_addr = match ip {
            core::net::Ipv4Addr::UNSPECIFIED => IpListenEndpoint { addr: None, port },
            _ => (ip, port).into(),
        };

        let (our_ch, their_ch) = Channel::new().unwrap();
        let (our_tx, our_rx) = our_ch.split();
        poll.add(
            our_rx.handle_id(),
            State::Listen(our_rx),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();

        {
            trace!("tcp-listen: {:?}", listen_addr);
            self.tcp_listen(listen_addr, our_tx).unwrap();
        }

        if let Err(err) = ch.send(Message::OpenReply {
            call_id,
            ch: their_ch,
        }) {
            debug_warn!("failed to send open reply message: {:?}", err);
        }
    }

    pub fn handle_listen_channel(
        &mut self,
        _poll: &Poll<State>,
        ch: &ChannelReceiver,
        msgbuffer: &mut MessageBuffer,
    ) {
        debug_warn!("got a message from a listen channel");
        let _ = ch.recv(msgbuffer);
    }

    pub fn handle_listen_close(&mut self, poll: &Poll<State>, ch: &ChannelReceiver) {
        debug_warn!("listen channel closed");
        poll.remove(ch.handle_id()).unwrap();
    }

    pub fn tcp_write(&mut self, poll: &Poll<State>, smol_handle: SocketHandle, data: &[u8]) {
        debug_warn!(
            "tcp_write: received {} bytes for socket {:?}",
            data.len(),
            smol_handle
        );
        match self.tcp_send(smol_handle, data) {
            Ok(written_len) => {
                debug_assert_eq!(written_len, data.len());
            }
            Err(err) => {
                debug_warn!("tcp_send failed: {:?}", err);
            }
        }

        self.poll(poll);
    }

    pub fn handle_data_close(
        &mut self,
        poll: &Poll<State>,
        ch: &ChannelReceiver,
        smol_handle: SocketHandle,
    ) {
        trace!("data channel closed for socket {:?}", smol_handle);
        if let Err(err) = self.close_socket(smol_handle) {
            debug_warn!("failed to close socket: {:?}", err);
        }

        poll.remove(ch.handle_id()).unwrap();
    }

    pub fn handle_data_channel(
        &mut self,
        poll: &Poll<State>,
        ch: &ChannelReceiver,
        smol_handle: SocketHandle,
        msgbuffer: &mut MessageBuffer,
    ) {
        let sendable_len = self.tcp_sendable_len(smol_handle).unwrap();
        if sendable_len < MESSAGE_DATA_LEN_MAX {
            debug_warn!(
                "TCP write buffer is almost full, throttling data channel {:?}",
                ch.handle_id()
            );

            poll.unlisten(ch.handle_id(), Readiness::READABLE | Readiness::CLOSED)
                .unwrap();
            self.mark_as_backpressured(smol_handle);
            return;
        }

        match ch.recv(msgbuffer) {
            Ok(Message::Data { data }) => {
                self.tcp_write(poll, smol_handle, data);
            }
            Ok(msg) => {
                debug_warn!("unexpected message on data channel: {:?}", msg);
            }
            Err(RecvError::Parse(msginfo)) => {
                debug_warn!("malformed message on data channel: {}", msginfo.kind());
            }
            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
            Err(RecvError::Syscall(err)) => {
                debug_warn!("recv error on data channel: {:?}", err);
            }
        }
    }

    pub fn receive_rx_packet(&mut self, poll: &Poll<State>, data: &[u8]) {
        self.receive_packet(data);
        self.poll(poll);
    }

    pub fn handle_driver_channel(
        &mut self,
        poll: &Poll<State>,
        ch: &ChannelReceiver,
        msgbuffer: &mut MessageBuffer,
    ) {
        match ch.recv(msgbuffer) {
            Ok(Message::Data { data }) => {
                self.receive_rx_packet(poll, data);
            }
            Ok(msg) => {
                debug_warn!("unexpected message on driver channel: {:?}", msg);
            }
            Err(RecvError::Parse(msginfo)) => {
                debug_warn!(
                    "malformed message on driver channel: {}",
                    msginfo.kind()
                );
            }
            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
            Err(RecvError::Syscall(err)) => {
                debug_warn!("recv error on driver channel: {:?}", err);
            }
        }
    }
}
