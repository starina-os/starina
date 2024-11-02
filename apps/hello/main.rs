#![no_std]
#![no_main]

use starina_api::environ::Environ;
use starina_api::prelude::*;

#[no_mangle]
pub fn main(_env: Environ) {
    info!("Hello World from hello app!");
}
