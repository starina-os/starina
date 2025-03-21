#![no_std]

pub mod autogen;
mod device;
mod smoltcp_logger;
mod tcpip;

use autogen::Env;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use starina::channel::ChannelSender;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;

pub struct App {
    driver: ChannelSender,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        smoltcp_logger::init();

        let driver = dispatcher
            .split_and_add_channel(env.driver)
            .expect("failed to get channel sender");

        info!("hello from tcpip");

        let ip = IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24);
        // let mac =

        Self { driver }
    }
}
