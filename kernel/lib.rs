#![no_std]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]
#![feature(fn_align)]

extern crate alloc;

#[macro_use]
mod print;

pub mod boot;
pub mod cpuvar;

mod arch;
mod channel;
mod device_tree;
mod folio;
mod handle;
mod interrupt;
mod memory;
mod panic;
mod poll;
mod process;
mod refcount;
mod scheduler;
mod signal;
mod spinlock;
mod startup;
mod syscall;
mod thread;
mod uaddr;
mod utils;
mod vmspace;
mod wait_queue;
