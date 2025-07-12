#![no_std]

mod device;
mod tcpip;

use core::net::Ipv4Addr;

use device::NetDevice;
use serde::Deserialize;
use smoltcp::iface::SocketHandle;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpListenEndpoint;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::channel::RecvError;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::MESSAGE_DATA_LEN_MAX;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use tcpip::SocketEvent;
use tcpip::TcpIp;

pub const SPEC: AppSpec = AppSpec {
    name: "tcpip",
    env: &[EnvItem {
        name: "driver",
        ty: EnvType::Service {
            service: "device/ethernet",
        },
    }],
    exports: &[ExportItem::Service { service: "tcpip" }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub startup_ch: Channel,
    pub driver: Channel,
}

fn parse_addr(addr: &str) -> Option<(Ipv4Addr, u16)> {
    let mut parts = addr.split(':');
    let ip = parts.next()?.parse().ok()?;
    let port = parts.next()?.parse().ok()?;
    Some((ip, port))
}

#[derive(Debug)]
enum State {
    Startup(Channel),
    Driver(ChannelReceiver),
    Control(Channel),
    Listen(ChannelReceiver),
    Data {
        smol_handle: SocketHandle,
        ch: ChannelReceiver,
    },
}

struct Mainloop<'a> {
    tcpip: TcpIp<'a>,
}

impl<'a> Mainloop<'a> {
    fn new(tcpip: TcpIp<'a>) -> Self {
        Self { tcpip }
    }

    fn handle_startup_connect(&mut self, poll: &Poll<State>, handle: Channel) {
        poll.add(
            handle.handle_id(),
            State::Control(handle),
            Readiness::READABLE | Readiness::CLOSED,
        )
        .unwrap();
    }

    fn handle_control_open(
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
            Ipv4Addr::UNSPECIFIED => IpListenEndpoint { addr: None, port },
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
            self.tcpip.tcp_listen(listen_addr, our_tx).unwrap();
        }

        if let Err(err) = ch.send(Message::OpenReply {
            call_id,
            handle: their_ch,
        }) {
            debug_warn!("failed to send open reply message: {:?}", err);
        }
    }

    fn tcp_write(&mut self, poll: &Poll<State>, smol_handle: SocketHandle, data: &[u8]) {
        debug_warn!(
            "tcp_write: received {} bytes for socket {:?}",
            data.len(),
            smol_handle
        );
        match self.tcpip.tcp_send(smol_handle, data) {
            Ok(written_len) => {
                debug_assert_eq!(written_len, data.len());
            }
            Err(err) => {
                debug_warn!("tcp_send failed: {:?}", err);
            }
        }

        self.tcpip_poll(poll);
    }

    fn handle_data_close(
        &mut self,
        poll: &Poll<State>,
        ch: &ChannelReceiver,
        smol_handle: SocketHandle,
    ) {
        trace!("data channel closed for socket {:?}", smol_handle);
        if let Err(err) = self.tcpip.close_socket(smol_handle) {
            debug_warn!("failed to close socket: {:?}", err);
        }

        // Remove channel from polling since it's closed, but socket will remain
        // in tcpip to complete graceful shutdown.
        poll.remove(ch.handle_id()).unwrap();
    }

    fn receive_rx_packet(&mut self, poll: &Poll<State>, data: &[u8]) {
        self.tcpip.receive_packet(data);
        self.tcpip_poll(poll);
    }

    fn tcpip_poll(&mut self, poll: &Poll<State>) {
        self.tcpip.poll(|ev| {
            match ev {
                SocketEvent::Data { ch, data } => {
                    ch.send(Message::Data { data }).unwrap();
                }
                SocketEvent::Closed { ch } => {
                    debug_warn!("socket fully closed, cleaning up channel");
                    if let Err(err) = poll.remove(ch.handle_id()) {
                        debug_warn!(
                            "failed to remove channel from poll (already removed?): {:?}",
                            err
                        );
                    }
                }
                SocketEvent::NewConnection { ch, smol_handle } => {
                    let (our_ch, their_ch) = Channel::new().unwrap();
                    let (our_tx, our_rx) = our_ch.split();
                    poll.add(
                        our_rx.handle_id(),
                        State::Data {
                            smol_handle,
                            ch: our_rx,
                        },
                        Readiness::READABLE | Readiness::CLOSED,
                    )
                    .expect("failed to get channel sender");

                    ch.send(Message::Connect { handle: their_ch }).unwrap();
                    *ch = our_tx;
                }
            }
        });

        for (ch_handle_id, smol_handle) in self.tcpip.get_writeable_sockets() {
            trace!(
                "write-backpressued socket is now writeable: {:?}",
                smol_handle
            );
            poll.listen(ch_handle_id, Readiness::READABLE | Readiness::CLOSED)
                .unwrap();
        }
    }
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).expect("failed to deserialize env");

    let poll = Poll::new().unwrap();
    poll.add(
        env.startup_ch.handle_id(),
        State::Startup(env.startup_ch),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let (driver_tx, driver_rx) = env.driver.split();
    poll.add(
        driver_rx.handle_id(),
        State::Driver(driver_rx),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let transmit = move |data: &[u8]| {
        trace!("transmit {} bytes", data.len());
        if let Err(err) = driver_tx.send(Message::Data { data }) {
            debug_warn!("failed to send: {:?}", err);
        }
    };

    let device = NetDevice::new(Box::new(transmit));
    let ip = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
    let gw_ip = Ipv4Addr::new(10, 0, 2, 2);
    let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

    let hwaddr = HardwareAddress::Ethernet(EthernetAddress(mac));
    let tcpip = TcpIp::new(device, ip, gw_ip, hwaddr);

    let mut mainloop = Mainloop::new(tcpip);
    let mut msgbuffer = MessageBuffer::new();
    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Connect { handle }) => {
                        mainloop.handle_startup_connect(&poll, handle);
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on startup channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!(
                            "unhandled message type on startup channel: {}",
                            msginfo.kind()
                        );
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on startup channel: {:?}", err);
                    }
                }
            }
            State::Startup(_) => {
                panic!("unexpected readiness for startup channel: {:?}", readiness);
            }
            State::Control(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Open { call_id, uri }) => {
                        mainloop.handle_control_open(&poll, ch, call_id, uri);
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on control channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!(
                            "unhandled message type on control channel: {}",
                            msginfo.kind()
                        );
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on control channel: {:?}", err);
                    }
                }
            }
            State::Control(ch) if readiness == Readiness::CLOSED => {
                debug_warn!("control channel closed");
                poll.remove(ch.handle_id()).unwrap();
            }
            State::Control(_) => {
                panic!("unexpected readiness for control channel: {:?}", readiness);
            }
            State::Listen(ch) if readiness.contains(Readiness::READABLE) => {
                debug_warn!("got a message from a listen channel");
                // Ignore any message.
                let _ = ch.recv(&mut msgbuffer);
            }
            State::Listen(ch) if readiness == Readiness::CLOSED => {
                debug_warn!("listen channel closed");
                poll.remove(ch.handle_id()).unwrap();
            }
            State::Listen(_) => {
                panic!("unexpected readiness for listen channel: {:?}", readiness);
            }
            State::Data { ch, smol_handle } if readiness.contains(Readiness::READABLE) => {
                let sendable_len = mainloop.tcpip.tcp_sendable_len(*smol_handle).unwrap();
                if sendable_len < MESSAGE_DATA_LEN_MAX {
                    debug_warn!(
                        "TCP write buffer is almost full, throttling data channel {:?}",
                        ch.handle_id()
                    );

                    // CLOSED is also ignored not to close the TCP socket.
                    poll.unlisten(ch.handle_id(), Readiness::READABLE | Readiness::CLOSED)
                        .unwrap();
                    mainloop.tcpip.mark_as_backpressured(*smol_handle);
                    continue;
                }

                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Data { data }) => {
                        mainloop.tcp_write(&poll, *smol_handle, data);
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on data channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!("unhandled message type on data channel: {}", msginfo.kind());
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on data channel: {:?}", err);
                    }
                }
            }
            State::Data { ch, smol_handle } if readiness == Readiness::CLOSED => {
                mainloop.handle_data_close(&poll, ch, *smol_handle);
            }
            State::Data { .. } => {
                panic!("unexpected readiness for data channel: {:?}", readiness);
            }
            State::Driver(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Data { data }) => {
                        mainloop.receive_rx_packet(&poll, data);
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on driver channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!(
                            "unhandled message type on driver channel: {}",
                            msginfo.kind()
                        );
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on driver channel: {:?}", err);
                    }
                }
            }
            State::Driver { .. } if readiness == Readiness::CLOSED => {
                panic!("driver channel closed");
            }
            State::Driver { .. } => {
                panic!("unexpected readiness for driver channel: {:?}", readiness);
            }
        }
    }
}
