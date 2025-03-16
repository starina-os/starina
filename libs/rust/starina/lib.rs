#![no_std]
#![cfg_attr(test, feature(test))]

extern crate alloc;

pub use starina_types::address;
pub use starina_types::error;

#[macro_use]
pub mod log;

pub mod channel;
pub mod eventloop;
pub mod folio;
pub mod handle;
pub mod message;
pub mod poll;
pub mod prelude;
pub mod syscall;
