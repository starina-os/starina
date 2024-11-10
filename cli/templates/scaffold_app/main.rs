#![no_std]
#![no_main]

use starina_api::environ::Environ;
use starina_api::prelude::*;

starina_api::autogen!();

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("Hello World!");
}
