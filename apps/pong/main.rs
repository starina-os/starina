#![no_std]
#![no_main]

use ftl_api::autogen::protocols::PingReply;
use ftl_api::autogen::protocols::PingRequest;
use ftl_api::channel::Channel;
use ftl_api::handle::OwnedHandle;
use ftl_api::prelude::*;
use ftl_api::types::handle::HandleId;
use ftl_api::types::message::MessageBuffer;

#[ftl_api::main]
pub fn main() {
    println!("[pong] starting pong");
    let handle_id = HandleId::from_raw(1);
    let handle = OwnedHandle::from_raw(handle_id);
    let ch = Channel::from_handle(handle);

    let mut buffer = MessageBuffer::new();
    for i in 0.. {
        println!("[pong] {}: receiving message", i);
        let r = ch
            .recv_with_buffer::<PingRequest>(&mut buffer)
            .expect("failed to recv");
        println!("[pong] {}: received message: {}", i, r.int_value1());

        println!("[pong] {}: replying message", i);
        ch.send_with_buffer(&mut buffer, PingReply { int_value2: 84 })
            .expect("failed to send");
    }
}
