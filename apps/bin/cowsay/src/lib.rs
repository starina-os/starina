#![no_std]

pub mod autogen;

use connection::ChannelWriter;
use connection::Conn;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;
use starina::sync::Mutex;

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        info!("cowsaying...");
        panic!("cowsaid");
    }
}
