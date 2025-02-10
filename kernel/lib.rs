#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(test, feature(test))]

extern crate alloc;

#[macro_use]
mod print;

mod allocator;
mod arch;
mod boot;
mod panic;
mod spinlock;

pub use boot::boot;
