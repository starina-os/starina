#![no_std]

pub mod autogen;

use autogen::Env;
use starina::channel::ChannelSender;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;

pub struct App {
    driver: ChannelSender,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        let driver = dispatcher
            .split_and_add_channel(env.driver)
            .expect("failed to get channel sender");

        info!("hello from tcpip");

        Self { driver }
    }
}
