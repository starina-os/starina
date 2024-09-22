#![no_std]
#![no_main]

use ftl_api::environ::Environ;
use ftl_api::prelude::*;

#[no_mangle]
pub fn main(_env: Environ) {
    info!("Hello World from hello app!");
}
