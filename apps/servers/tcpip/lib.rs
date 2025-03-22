#![no_std]

pub mod autogen;
mod device;
mod smoltcp_logger;
mod tcpip;

use autogen::Env;
use device::NetDevice;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;
use tcpip::TcpIp;

pub struct App {
    tcpip: TcpIp<'static>,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        smoltcp_logger::init();

        let driver = dispatcher
            .split_and_add_channel(env.driver)
            .expect("failed to get channel sender");

        info!("hello from tcpip");

        let transmit = move |buf: &[u8]| {
            todo!("transmit: {:?}", buf);
        };

        // FIXME:
        let device = NetDevice::new(Box::new(transmit));
        let ip = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
        let mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let mut tcpip = TcpIp::new(device, ip, HardwareAddress::Ethernet(EthernetAddress(mac)));

        tcpip.poll(|ev| {
            trace!("event: {:?}", ev);
        });

        Self { tcpip }
    }
}
