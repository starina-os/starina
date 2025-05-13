#![no_std]

pub mod autogen;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;

pub enum State {}

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        info!("cowsaying...");

        starina_linux::Command::new("cowsay")
            .arg("-f")
            .arg("dragon")
            .spawn()
            .expect("failed to execute process");

        panic!("cowsaid");
    }
}
