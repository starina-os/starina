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
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::Message;
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
    Listen,
    Data {
        smol_handle: SocketHandle,
        ch: ChannelReceiver,
    },
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
        driver_rx.handle().id(),
        State::Driver(driver_rx),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let transmit = move |data: &[u8]| {
        trace!("transmit {} bytes", data.len());
        if let Err(err) = driver_tx.send(Message::FramedData { data }) {
            debug_warn!("failed to send: {:?}", err);
        }
    };

    // FIXME:
    let device = NetDevice::new(Box::new(transmit));
    let ip = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
    let gw_ip = Ipv4Addr::new(10, 0, 2, 2);
    let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

    let hwaddr = HardwareAddress::Ethernet(EthernetAddress(mac));
    let mut tcpip = TcpIp::new(device, ip, gw_ip, hwaddr);
    'mainloop: loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::Connect { handle }) => {
                        poll.add(
                            handle.handle_id(),
                            State::Control(handle),
                            Readiness::READABLE | Readiness::CLOSED,
                        )
                        .unwrap();
                    }
                    _ => {
                        debug_warn!("unhandled message: {:?}", m.msginfo);
                    }
                }
            }
            State::Startup(_) => {
                panic!("unexpected readiness for startup channel: {:?}", readiness);
            }
            State::Control(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::Open { call_id, uri }) => {
                        let uri = core::str::from_utf8(uri).unwrap();
                        info!("got open message: {}", uri);
                        let Some(("tcp-listen", rest)) = uri.split_once(':') else {
                            ch.send(Message::Abort {
                                call_id,
                                reason: ErrorCode::InvalidUri,
                            })
                            .unwrap();
                            continue 'mainloop;
                        };

                        let Some((ip, port)) = parse_addr(rest) else {
                            debug_warn!("invalid tcp-listen message: {}", uri);
                            ch.send(Message::Abort {
                                call_id,
                                reason: ErrorCode::InvalidUri,
                            })
                            .unwrap();
                            continue 'mainloop;
                        };

                        let listen_addr = match ip {
                            Ipv4Addr::UNSPECIFIED => IpListenEndpoint { addr: None, port },
                            _ => (ip, port).into(),
                        };

                        let (our_ch, their_ch) = Channel::new().unwrap();
                        let (our_tx, our_rx) = our_ch.split();
                        poll.add(
                            our_rx.handle().id(),
                            State::Listen,
                            Readiness::READABLE | Readiness::CLOSED,
                        )
                        .unwrap();

                        // Have a separate scope to drop the tcpip lock as soon as possible.
                        {
                            trace!("tcp-listen: {:?}", listen_addr);
                            tcpip.tcp_listen(listen_addr, our_tx).unwrap();
                        }

                        if let Err(err) = ch.send(Message::OpenReply {
                            call_id,
                            handle: their_ch,
                        }) {
                            debug_warn!("failed to send open reply message: {:?}", err);
                        }
                    }
                    _ => {
                        debug_warn!("unhandled message: {:?}", m.msginfo);
                    }
                }
            }
            State::Control(_) if readiness == Readiness::CLOSED => {
                debug_warn!("control channel closed");
                break 'mainloop;
            }
            State::Control(_) => {
                panic!("unexpected readiness for control channel: {:?}", readiness);
            }
            State::Listen => {
                debug_warn!("got a message from a listen channel");
            }
            State::Data { ch, smol_handle } if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::StreamData { data }) => {
                        // FIXME: backpressure
                        tcpip.tcp_send(*smol_handle, data).unwrap();
                        tcpip_poll(&poll, &mut tcpip);
                    }
                    _ => {
                        debug_warn!("unhandled message: {:?}", m.msginfo);
                    }
                }
            }
            State::Data { ch, smol_handle } if readiness == Readiness::CLOSED => {
                debug_warn!("data channel closed for socket {:?}", smol_handle);
                poll.remove(ch.handle().id()).unwrap();
            }
            State::Data { .. } => {
                panic!("unexpected readiness for data channel: {:?}", readiness);
            }
            State::Driver(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::FramedData { data }) => {
                        tcpip.receive_packet(data);
                        tcpip_poll(&poll, &mut tcpip);
                    }
                    _ => {
                        debug_warn!("unhandled message: {:?}", m.msginfo);
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

fn tcpip_poll<'a>(poll: &Poll<State>, tcpip: &mut TcpIp<'a>) {
    tcpip.poll(|ev| {
        match ev {
            SocketEvent::Data { ch, data } => {
                ch.send(Message::StreamData { data }).unwrap(); // FIXME: what if backpressure happens?
            }
            SocketEvent::Close { ch } => {
                debug_warn!("closing a socket");
                poll.remove(ch.handle().id()).unwrap();
            }
            SocketEvent::NewConnection { ch, smol_handle } => {
                let (our_ch, their_ch) = Channel::new().unwrap();
                let (our_tx, our_rx) = our_ch.split();
                poll.add(
                    our_rx.handle().id(),
                    State::Data {
                        smol_handle,
                        ch: our_rx,
                    },
                    Readiness::READABLE | Readiness::CLOSED,
                )
                .expect("failed to get channel sender");

                ch.send(Message::Connect { handle: their_ch }).unwrap(); // FIXME: what if backpressure happens?

                // The socket has become an esblished socket, so replace the old
                // sender handle with a new data channel.
                *ch = our_tx;
            }
        }
    });
}
