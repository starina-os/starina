#![no_std]
#![no_main]

starina_api::autogen!();

use starina_api::environ::Environ;
use starina_api::prelude::*;
use starina_api::types::message::MessageBuffer;
use starina_autogen::idl::echo::Ping;

#[no_mangle]
pub fn main(mut env: Environ) {
    let echo_ch = env.take_channel("dep:echo").unwrap();

    let mut value: i32 = 0;
    loop {
        value = value.saturating_add(1);

        let mut msgbuffer = MessageBuffer::new();
        let reply = echo_ch.call(&mut msgbuffer, Ping { value }).unwrap();
        info!("received: {:?}", reply);
    }
}
