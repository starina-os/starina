#![no_std]

pub mod autogen;
mod cowsay;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;

pub enum State {}

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(_dispatcher: &dyn Dispatcher<Self::State>, _env: Self::Env) -> Self {
        info!("cowsaying...");
        cowsay::cowsay("Hello from Cow!");
        panic!("cowsaid");
    }
}
