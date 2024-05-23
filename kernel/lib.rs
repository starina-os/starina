#![no_std]

extern crate alloc;

#[macro_use]
mod print;

pub mod boot;
pub mod cpuvar;

mod arch;
mod memory;
mod panic;
mod spinlock;
