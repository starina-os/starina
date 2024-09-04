#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::environ::Environ;
use ftl_api::prelude::*;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::protocols::ping::PingReply;
use ftl_api_autogen::protocols::ping::PingRequest;

#[no_mangle]
pub fn main(mut env: Environ) {
    let ch = env.take_channel("dep:ping_server").unwrap();
    let mut msgbuffer = MessageBuffer::new();
    loop {
        ch.send(PingRequest { value: 42 }).unwrap();

        let reply = ch.recv_with_buffer::<PingReply>(&mut msgbuffer).unwrap();
        info!("received {}", reply.value());
    }
}
