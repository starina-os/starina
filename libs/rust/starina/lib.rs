#![no_std]
#![cfg_attr(test, feature(test))]

extern crate alloc;

pub use starina_types::address;
pub use starina_types::error;
pub use starina_types::message;

#[macro_use]
pub mod log;

pub mod app;
pub mod channel;
pub mod handle;
pub mod poll;
pub mod syscall;
