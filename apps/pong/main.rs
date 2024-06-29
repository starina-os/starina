#![no_std]
#![no_main]

use ftl_api::channel::Channel;
use ftl_api::handle::OwnedHandle;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::message::MessageBuffer;
use ftl_api_autogen::apps::pong::Environ;
use ftl_api_autogen::apps::pong::Message;
use ftl_api_autogen::protocols::ping::PingReply;

enum Context {
    Autopilot,
    Client { counter: i32 },
}

#[ftl_api::main]
pub fn main(mut env: Environ) {
    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    mainloop
        .add_channel(env.autopilot_ch.take().unwrap(), Context::Autopilot)
        .unwrap();

    let mut buffer = MessageBuffer::new();
    loop {
        match mainloop.next(&mut buffer) {
            Event::Message { ch, ctx, m } => {
                match (ctx, m) {
                    (Context::Autopilot, Message::NewclientRequest(m)) => {
                        println!("got new client: {:?}", m.handle());
                        let new_ch = Channel::from_handle(OwnedHandle::from_raw(m.handle()));
                        mainloop
                            .add_channel(new_ch, Context::Client { counter: 0 })
                            .unwrap();
                    }
                    (Context::Client { counter }, Message::PingRequest(m)) => {
                        println!("[pong] received message: {}", m.int_value1());
                        *counter += 1;

                        let reply = PingReply {
                            int_value2: *counter,
                        };
                        if let Err(err) = ch.send_with_buffer(&mut buffer, reply) {
                            println!("failed to reply: {:?}", err);
                        }
                    }
                    _ => {
                        // TODO: dump message with fmt::Debug
                        panic!("unknown message");
                    }
                }
            }
            Event::Error(err) => {
                panic!("mainloop error: {:?}", err);
            }
        }
    }
}
