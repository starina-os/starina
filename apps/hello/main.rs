#![no_std]
#![no_main]

use ftl_api::{environ::Environ, prelude::*, println};

#[ftl_api::main]
pub fn main(_env: Environ) {
    println!("Hello World from hello app!");
    loop {}
}
