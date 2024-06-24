#![no_std]
#![no_main]

use ftl_api::channel::Channel;
use ftl_api::handle::OwnedHandle;
use ftl_api::message::MessageBuffer;
use ftl_api::message::PingPongMessage;
use ftl_api::prelude::*;
use ftl_api::types::handle::HandleId;

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
            .recv_with_buffer::<PingPongMessage>(&mut buffer)
            .expect("failed to recv");
        println!("[pong] {}: received message: {}", i, r.value());

        println!("[pong] {}: replying message", i);
        ch.send_with_buffer(&mut buffer, PingPongMessage { value: 84 })
            .expect("failed to send");
    }
}
