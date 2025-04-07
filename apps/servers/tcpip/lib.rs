#![no_std]

pub mod autogen;
mod device;
mod smoltcp_logger;
mod tcpip;

use core::net::Ipv4Addr;

use autogen::Env;
use device::NetDevice;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use starina::channel::Channel;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::message::ConnectMsg;
use starina::message::FramedDataMsg;
use starina::message::OpenMsg;
use starina::message::OpenReplyMsg;
use starina::message::StreamDataMsg;
use starina::prelude::*;
use tcpip::SocketEvent;
use tcpip::TcpIp;

fn parse_addr(addr: &str) -> Option<(Ipv4Addr, u16)> {
    let mut parts = addr.split(':');
    let ip = parts.next()?.parse().ok()?;
    let port = parts.next()?.parse().ok()?;
    Some((ip, port))
}

pub struct App {
    tcpip: spin::Mutex<TcpIp<'static>>,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        smoltcp_logger::init();

        let driver = dispatcher
            .split_and_add_channel(env.driver)
            .expect("failed to get channel sender");

        info!("hello from tcpip");

        let transmit = move |data: &[u8]| {
            trace!("transmit {} bytes", data.len());
            if let Err(err) = driver.send(FramedDataMsg { data }) {
                debug_warn!("failed to send: {:?}", err);
            }
        };

        // FIXME:
        let device = NetDevice::new(Box::new(transmit));
        let ip = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
        let gw_ip = Ipv4Addr::new(10, 0, 2, 2);
        let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

        let hwaddr = HardwareAddress::Ethernet(EthernetAddress(mac));
        let tcpip = TcpIp::new(device, ip, gw_ip, hwaddr);

        // let (our_ch, their_ch) = Channel::new().unwrap();
        // let (sender, receiver) = our_ch.split();

        // let remote_endpoint = (IpAddress::v4(96, 7, 181, 39), 80).into();
        // let sock = tcpip.tcp_connect(remote_endpoint, sender).unwrap();

        // trace!("polling");
        // tcpip.poll(|ev| {
        //     trace!("event: {:?}", ev);
        // });
        // trace!("polling done");

        Self {
            tcpip: spin::Mutex::new(tcpip),
        }
    }

    fn on_open(&self, ctx: &Context, msg: OpenMsg<'_>) {
        match msg.uri.split_once(':') {
            Some(("tcp-listen", rest)) => {
                let Some((ip, port)) = parse_addr(rest) else {
                    debug_warn!("invalid tcp-listen message: {}", msg.uri);
                    return;
                };

                let listen_addr = (ip, port).into();
                let (our_ch, their_ch) = Channel::new().unwrap();
                let sender = ctx.dispatcher.split_and_add_channel(our_ch).unwrap();

                // Have a separate scope to drop the tcpip lock as soon as possible.
                {
                    trace!("tcp-listen: {:?}", listen_addr);
                    let mut tcpip = self.tcpip.lock();
                    tcpip.tcp_listen(listen_addr, sender).unwrap();
                }

                ctx.sender.send(OpenReplyMsg { handle: their_ch }).unwrap(); // FIXME: what if backpressure happens?
            }
            _ => {
                debug_warn!("unknown open message: {}", msg.uri);
                // FIXME: How should we reply error?
            }
        }
    }

    fn on_framed_data(&self, ctx: &Context, msg: FramedDataMsg<'_>) {
        trace!("polling");
        self.tcpip.lock().receive_packet(msg.data);

        self.tcpip.lock().poll(ctx, |ctx, ev| {
            match ev {
                SocketEvent::Data { ch, data } => {
                    ch.send(StreamDataMsg { data }).unwrap(); // FIXME: what if backpressure happens?
                }
                SocketEvent::Close { ch } => {
                    warn!("SocketEvent::Close: not yet implemented");
                }
                SocketEvent::NewConnection { ch, smol_handle } => {
                    let (ours, theirs) = Channel::new().unwrap();
                    let our_ch_sender = ctx
                        .dispatcher
                        .split_and_add_channel(ours)
                        .expect("failed to get channel sender");
                    ch.send(ConnectMsg { handle: theirs }).unwrap(); // FIXME: what if backpressure happens?

                    // The socket has become an esblished socket, so replace the old
                    // sender handle with a new data channel.
                    *ch = our_ch_sender;
                }
            }
        });
        trace!("polling done");

        core::mem::forget(msg);
    }
}
