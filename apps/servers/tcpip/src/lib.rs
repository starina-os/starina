#![no_std]

pub mod autogen;
mod device;
mod smoltcp_logger;
mod tcpip;

use core::net::Ipv4Addr;

use device::NetDevice;
use smoltcp::iface::SocketHandle;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpListenEndpoint;
use starina::channel::Channel;
use starina::error::ErrorCode;
use starina::eventloop::Completer;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::message::ConnectMsg;
use starina::message::ErrorMsg;
use starina::message::FramedDataMsg;
use starina::message::OpenMsg;
use starina::message::OpenReplyMsg;
use starina::message::StreamDataMsg;
use starina::prelude::*;
use starina::sync::Mutex;
use tcpip::SocketEvent;
use tcpip::TcpIp;

fn parse_addr(addr: &str) -> Option<(Ipv4Addr, u16)> {
    let mut parts = addr.split(':');
    let ip = parts.next()?.parse().ok()?;
    let port = parts.next()?.parse().ok()?;
    Some((ip, port))
}

#[derive(Debug)]
pub enum State {
    Startup,
    Driver,
    Control,
    Listen,
    Data { smol_handle: SocketHandle },
}

pub struct App<'a> {
    tcpip: Mutex<TcpIp<'a>>,
}

impl<'a> EventLoop for App<'a> {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        smoltcp_logger::init();

        dispatcher
            .add_channel(State::Startup, env.startup_ch)
            .unwrap();

        let driver = dispatcher.add_channel(State::Driver, env.driver).unwrap();

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

        Self {
            tcpip: Mutex::new(tcpip),
        }
    }

    fn on_connect(&self, ctx: Context<Self::State>, msg: ConnectMsg) {
        ctx.dispatcher
            .add_channel(State::Control, msg.handle)
            .unwrap();
    }

    fn on_open(
        &self,
        ctx: Context<Self::State>,
        completer: Completer<OpenReplyMsg>,
        msg: OpenMsg<'_>,
    ) {
        let uri = core::str::from_utf8(msg.uri).unwrap();
        info!("got open message: {}", uri);
        let Some(("tcp-listen", rest)) = uri.split_once(':') else {
            completer.abort(ErrorCode::InvalidUri);
            return;
        };

        let Some((ip, port)) = parse_addr(rest) else {
            debug_warn!("invalid tcp-listen message: {}", uri);
            completer.abort(ErrorCode::InvalidUri);
            return;
        };

        let listen_addr = match ip {
            Ipv4Addr::UNSPECIFIED => IpListenEndpoint { addr: None, port },
            _ => (ip, port).into(),
        };

        let (our_ch, their_ch) = Channel::new().unwrap();
        let sender = ctx.dispatcher.add_channel(State::Listen, our_ch).unwrap();

        // Have a separate scope to drop the tcpip lock as soon as possible.
        {
            trace!("tcp-listen: {:?}", listen_addr);
            let mut tcpip = self.tcpip.lock();
            tcpip.tcp_listen(listen_addr, sender).unwrap();
        }

        if let Err(err) = completer.reply(OpenReplyMsg { handle: their_ch }) {
            debug_warn!("failed to send open reply message: {:?}", err);
        }
    }

    fn on_stream_data(&self, ctx: Context<Self::State>, msg: StreamDataMsg<'_>) {
        let State::Data { smol_handle } = &ctx.state else {
            debug_warn!("stream data from unexpected state: {:?}", ctx.state);
            if let Err(err) = ctx.sender.send(ErrorMsg {
                reason: ErrorCode::InvalidState,
            }) {
                debug_warn!("failed to send error message: {:?}", err);
            }
            return;
        };

        let mut tcpip = self.tcpip.lock();
        // FIXME: backpressure
        tcpip.tcp_send(*smol_handle, msg.data).unwrap();
        poll(ctx.dispatcher, &mut tcpip);
    }

    fn on_framed_data(&self, ctx: Context<Self::State>, msg: FramedDataMsg<'_>) {
        let mut tcpip = self.tcpip.lock();
        tcpip.receive_packet(msg.data);
        poll(ctx.dispatcher, &mut tcpip);
    }
}

fn poll<'a>(dispatcher: &dyn Dispatcher<State>, tcpip: &mut TcpIp<'a>) {
    tcpip.poll(dispatcher, |dispatcher, ev| {
        match ev {
            SocketEvent::Data { ch, data } => {
                ch.send(StreamDataMsg { data }).unwrap(); // FIXME: what if backpressure happens?
            }
            SocketEvent::Close { ch } => {
                dispatcher.close_channel(ch.handle().id()).unwrap();
            }
            SocketEvent::NewConnection { ch, smol_handle } => {
                let (ours, theirs) = Channel::new().unwrap();
                let our_ch_sender = dispatcher
                    .add_channel(State::Data { smol_handle }, ours)
                    .expect("failed to get channel sender");

                ch.send(ConnectMsg { handle: theirs }).unwrap(); // FIXME: what if backpressure happens?

                // The socket has become an esblished socket, so replace the old
                // sender handle with a new data channel.
                *ch = our_ch_sender;
            }
        }
    });
}
