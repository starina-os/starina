#![no_std]
#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate alloc;

pub mod headers;
pub mod method;
pub mod request;
