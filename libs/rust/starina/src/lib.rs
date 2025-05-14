#![no_std]
#![feature(pointer_is_aligned_to)]
#![cfg_attr(test, feature(test))]

extern crate alloc;

pub use starina_types::address;
pub use starina_types::device_tree;
pub use starina_types::error;
pub use starina_types::spec;

#[macro_use]
pub mod log;

pub mod channel;
pub mod collections;
pub mod eventloop;
pub mod folio;
pub mod handle;
pub mod interrupt;
pub mod iobus;
pub mod message;
pub mod poll;
pub mod sync;

pub mod prelude;
pub mod syscall;
pub mod tls;
pub mod vmspace;
