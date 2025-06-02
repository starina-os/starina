#![no_std]
#![feature(naked_functions)]

pub mod autogen;

use core::arch::naked_asm;
use core::mem::offset_of;

use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::handle::HandleId;
use starina::prelude::*;
use starina::syscall::thread_create;

#[derive(Debug)]
pub enum State {}

pub struct App {}

struct ThreadArg {
    entry: usize,
    sp_top: usize,
}

#[unsafe(naked)]
extern "C" fn thread_entry() -> ! {
    // a0 points to *const ThreadArg.
    naked_asm!(
        // Load the stack pointer from the ThreadArg.
        "ld sp, {stack_offset}(a0)",
        "ld a1, {entry_offset}(a0)",
        "jalr a1",
        stack_offset = const offset_of!(ThreadArg, sp_top),
        entry_offset = const offset_of!(ThreadArg, entry),
    );
}

fn true_entry() -> ! {
    // This is the entry point for the thread.
    // It should be defined in the application code.
    loop {
        // Simulate some work.
        for i in 0.. {
            if i % 100000 == 0 {
                starina::syscall::console_write(b"B");
            }
        }
        starina::syscall::thread_exit();
    }
}

fn do_main() {
    info!("running automated tests...");
    let stack = vec![0; 1024 * 1024];
    let sp_top = stack.as_ptr() as usize + stack.len();
    let arg = Box::into_raw(Box::new(ThreadArg {
        sp_top,
        entry: true_entry as usize,
    }));
    let thread = thread_create(HandleId::from_raw(0), thread_entry as usize, arg as usize);
    for i in 0.. {
        if i % 100000 == 0 {
            starina::syscall::console_write(b"A");
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
