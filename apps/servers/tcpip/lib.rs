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
use starina::message::FramedData;
use starina::message::Message;
use starina::prelude::*;
use tcpip::SocketEvent;
use tcpip::TcpIp;

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
            if let Err(err) = driver.send(FramedData { data }) {
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

        let (our_ch, their_ch) = Channel::new().unwrap();
        let (sender, receiver) = our_ch.split();

        let remote_endpoint = (IpAddress::v4(96, 7, 181, 39), 80).into();
        let sock = tcpip.tcp_connect(remote_endpoint, sender).unwrap();

        trace!("polling");
        tcpip.poll(|ev| {
            trace!("event: {:?}", ev);
        });
        trace!("polling done");

        Self {
            tcpip: spin::Mutex::new(tcpip),
        }
    }

    fn on_framed_data(&self, _ctx: &Context, msg: Message<FramedData<'_>>) {
        trace!("frame data received: {:2x?}", msg.data());
        self.tcpip.lock().receive_packet(msg.data());
        trace!("polling");
        self.tcpip.lock().poll(|ev| {
            match ev {
                SocketEvent::Data { ch, data } => {
                    trace!("\n\x1b[1;37m{:?}\x1b[0m", core::str::from_utf8(data));
                }
                SocketEvent::Close { ch } => {
                    todo!()
                }
                SocketEvent::NewConnection { ch, smol_handle } => {
                    todo!()
                }
            }
        });
        trace!("polling done");
    }
}
