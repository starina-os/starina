#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]
#![feature(fn_align)]

extern crate alloc;

#[macro_use]
mod print;

pub mod boot;
pub mod cpuvar;

mod app_loader;
mod arch;
mod autopilot;
mod bootfs;
mod channel;
mod device_tree;
mod folio;
mod signal;
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
