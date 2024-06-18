#![no_std]
#![no_main]

use ftl_api::prelude::*;

extern crate ftl_api;

#[no_mangle]
pub fn main() {
    println!("Hello World from hello app!");
    loop {}
}
