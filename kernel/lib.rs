#![no_std]
#![feature(asm_const)]
#![feature(effects)]
#![feature(const_trait_impl)]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]

extern crate alloc;

#[macro_use]
mod print;

pub mod boot;
pub mod cpuvar;

mod app_loader;
mod arch;
mod autopilot;
mod buffer;
mod channel;
mod handle;
mod memory;
mod panic;
mod poll;
mod process;
mod ref_counted;
mod scheduler;
mod sleep;
mod spinlock;
mod syscall;
mod thread;
