#![no_std]
#![cfg_attr(test, feature(test))]

extern crate alloc;

pub use starina_types::address;
pub use starina_types::device_tree;
pub use starina_types::environ;
pub use starina_types::error;
pub use starina_types::spec;

#[macro_use]
pub mod log;

pub use alloc::borrow;

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::trace;
pub use log::warn;

pub mod channel;
pub mod collections;
pub mod folio;
pub mod handle;
pub mod hvspace;
pub mod interrupt;
pub mod mainloop;
pub mod message;
pub mod mmio;
pub mod poll;
pub mod start;
pub mod sync;
pub mod thread;
pub mod timer;
pub mod vcpu;

pub mod prelude;
pub mod syscall;
pub mod tls;
pub mod vmspace;
