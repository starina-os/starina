#![no_std]

pub mod autogen;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::prelude::*;
use starina::thread::Thread;

#[derive(Debug)]
pub enum State {}

pub struct App {}

fn do_main() {
    info!("running automated tests...");
    Thread::spawn(|| {
        for i in 0.. {
            if i % 100000 == 0 {
                starina::syscall::console_write(b"A");
            }
        }
    })
    .unwrap();
    Thread::spawn(|| {
        for i in 0.. {
            if i % 100000 == 0 {
                starina::syscall::console_write(b"B");
            }
        }
    })
    .unwrap();

    for i in 0.. {
        if i % 100000 == 0 {
            starina::syscall::console_write(b"C");
        }
    }
}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        do_main();
        todo!()
    }
}
