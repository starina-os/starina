#![no_std]
#![no_main]

use ftl_api::autogen::protocols::PingReply;
use ftl_api::autogen::protocols::PingRequest;
use ftl_api::channel::Channel;
use ftl_api::handle::OwnedHandle;
use ftl_api::poll::Poll;
use ftl_api::prelude::*;
use ftl_api::types::handle::HandleId;
use ftl_api::types::message::MessageBuffer;
use ftl_api::types::poll::PollEvent;

#[ftl_api::main]
pub fn main() {
    println!("[pong] starting pong");
    let ch = {
        let handle_id = HandleId::from_raw(1);
        let handle = OwnedHandle::from_raw(handle_id);
        Channel::from_handle(handle)
    };

    let poll = {
        let handle_id = HandleId::from_raw(2);
        let handle = OwnedHandle::from_raw(handle_id);
        Poll::from_handle(handle)
    };

    println!("[pong] adding poll entry");
    poll.add(ch.handle().id(), PollEvent::READABLE).unwrap();

    let mut buffer = MessageBuffer::new();
    for i in 0.. {
        println!("[pong] {}: polling", i);
        let (ev, handle_id) = poll.wait().unwrap();
        if handle_id != ch.handle().id() {
            println!("[pong] unexpected handle id: {:?}", handle_id);
        }

        if !ev.contains(PollEvent::READABLE) {
            println!("[pong] unexpected event: {:?}", ev);
        }

        println!("[pong] {}: receiving message", i);
        let r = ch
            .recv_with_buffer::<PingRequest>(&mut buffer)
            .expect("failed to recv");
        println!("[ping] {}: received message: {}", i, r.int_value1());
        println!("[pong] {}: replying message", i);
        ch.send_with_buffer(&mut buffer, PingReply { int_value2: 84 })
            .expect("failed to send");
    }
}
