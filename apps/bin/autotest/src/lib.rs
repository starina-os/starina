#![no_std]

pub mod autogen;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;

#[derive(Debug)]
pub enum State {}

pub struct App {}

fn do_main() {
    info!("running automated tests...");
}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        do_main();
        todo!()
    }
}
