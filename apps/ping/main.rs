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
    println!("[ping] starting ping");
    let handle_id = HandleId::from_raw(1);
    let handle = OwnedHandle::from_raw(handle_id);
    let ch = Channel::from_handle(handle);

    let mut buffer = MessageBuffer::new();
    for i in 0.. {
        println!("[ping] {}: sending message", i);
        ch.send_with_buffer(&mut buffer, PingPongMessage { value: 42 })
            .unwrap();

        println!("[ping] {}: receiving message", i);
        let r = ch.recv_with_buffer::<PingPongMessage>(&mut buffer).unwrap();
        println!("[ping] {}: received message: {}", i, r.value());
    }
}
