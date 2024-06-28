#![no_std]
#![no_main]

use ftl_api::prelude::*;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::apps::ping::Environ;
use ftl_api_autogen::protocols::ping::PingReply;
use ftl_api_autogen::protocols::ping::PingRequest;

#[ftl_api::main]
pub fn main(mut env: Environ) {
    println!("[ping] starting ping");
    let ch = env.depends.ping_server.take().unwrap();

    let mut buffer = MessageBuffer::new();
    for i in 0.. {
        println!("[ping] {}: sending message", i);
        ch.send_with_buffer(&mut buffer, PingRequest { int_value1: 42 })
            .unwrap();

        println!("[ping] {}: receiving message", i);
        let r = ch.recv_with_buffer::<PingReply>(&mut buffer).unwrap();
        println!("[ping] {}: received message: {}", i, r.int_value2());
    }
}
