#![no_std]

mod device;
mod tcpip;

use core::net::Ipv4Addr;

use device::NetDevice;
use serde::Deserialize;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::HardwareAddress;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::iface::SocketHandle;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::environ::Environ;
use starina::handle::Handleable;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use tcpip::TcpIp;

#[derive(Debug)]
pub enum State {
    Startup(Channel),
    Driver(ChannelReceiver),
    Control(Channel),
    Listen(ChannelReceiver),
    Data {
        smol_handle: SocketHandle,
        ch: ChannelReceiver,
    },
}

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

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("failed to deserialize env");

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
    let mut tcpip = TcpIp::new(device, ip, gw_ip, hwaddr);
    let mut msgbuffer = MessageBuffer::new();
    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) if readiness.contains(Readiness::READABLE) => {
                tcpip.handle_startup_channel(&poll, ch, &mut msgbuffer);
            }
            State::Startup(_) => {
                panic!("unexpected readiness for startup channel: {:?}", readiness);
            }
            State::Control(ch) if readiness.contains(Readiness::READABLE) => {
                tcpip.handle_control_channel(&poll, ch, &mut msgbuffer);
            }
            State::Control(ch) if readiness == Readiness::CLOSED => {
                tcpip.handle_control_close(&poll, ch);
            }
            State::Control(_) => {
                panic!("unexpected readiness for control channel: {:?}", readiness);
            }
            State::Listen(ch) if readiness.contains(Readiness::READABLE) => {
                tcpip.handle_listen_channel(&poll, ch, &mut msgbuffer);
            }
            State::Listen(ch) if readiness == Readiness::CLOSED => {
                tcpip.handle_listen_close(&poll, ch);
            }
            State::Listen(_) => {
                panic!("unexpected readiness for listen channel: {:?}", readiness);
            }
            State::Data { ch, smol_handle } if readiness.contains(Readiness::READABLE) => {
                tcpip.handle_data_channel(&poll, ch, *smol_handle, &mut msgbuffer);
            }
            State::Data { ch, smol_handle } if readiness == Readiness::CLOSED => {
                tcpip.handle_data_close(&poll, ch, *smol_handle);
            }
            State::Data { .. } => {
                panic!("unexpected readiness for data channel: {:?}", readiness);
            }
            State::Driver(ch) if readiness.contains(Readiness::READABLE) => {
                tcpip.handle_driver_channel(&poll, ch, &mut msgbuffer);
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
