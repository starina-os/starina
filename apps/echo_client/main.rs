#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::environ::Environ;
use ftl_api::prelude::*;
use ftl_api::types::message::MessageBuffer;
use ftl_autogen::idl::echo::Ping;

#[no_mangle]
pub fn main(mut env: Environ) {
    let echo_ch = env.take_channel("dep:echo").unwrap();

    let mut value: i32 = 0;
    loop {
        value = value.saturating_add(1);

        let mut msgbuffer = MessageBuffer::new();
        let reply = echo_ch
            .call_with_buffer(Ping { value }, &mut msgbuffer)
            .unwrap();
        info!("received: {}", reply.value);
    }
}
