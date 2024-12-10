#![no_std]

extern crate alloc;

#[macro_use]
pub mod print;

mod panic;
mod start;

pub mod allocator;
pub mod arch;
pub mod handle;
pub mod syscall;

pub use starina_types::error;
