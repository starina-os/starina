#![no_std]
#![no_main]

use ftl_api::environ::Environ;
use ftl_api::prelude::*;

#[ftl_api::main]
pub fn main(_env: Environ) {
    info!("Hello World from hello app!");
    loop {}
}
