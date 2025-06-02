#![no_std]

pub mod autogen;
mod catsay;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::syscall;

pub enum State {}

pub struct App {}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(_dispatcher: &dyn Dispatcher<Self::State>, _env: Self::Env) -> Self {
        catsay::catsay("I'm a teapot!");
        syscall::thread_exit();
    }
}
