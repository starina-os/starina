#![no_std]
#![feature(asm_const)]
#![feature(effects)]
#![feature(const_trait_impl)]
#![feature(offset_of)]
#![feature(naked_functions)]
#![feature(unsize)]
#![feature(coerce_unsized)]

extern crate alloc;

#[macro_use]
mod print;

pub mod boot;
pub mod cpuvar;

mod arch;
mod handle;
mod app_loader;
mod memory;
mod panic;
mod process;
mod ref_counted;
mod scheduler;
mod spinlock;
mod syscall;
mod thread;
