#![no_std]

extern crate alloc;

#[macro_use]
mod print;

mod allocator;
mod arch;
mod boot;
mod panic;
mod spinlock;

pub use boot::boot;
