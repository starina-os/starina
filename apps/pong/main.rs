#![no_std]
#![no_main]

use ftl_api::autogen::protocols::PingReply;
use ftl_api::autogen::protocols::PingRequest;
use ftl_api::channel::Channel;
use ftl_api::handle::OwnedHandle;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::handle::HandleId;
use ftl_api::types::message::MessageBuffer;

struct State {
    counter: i32,
}

#[ftl_api::main]
pub fn main() {

    let ch = {
        let handle_id = HandleId::from_raw(1);
        let handle = OwnedHandle::from_raw(handle_id);
        Channel::from_handle(handle)
    };

    let mut mainloop = Mainloop::<State, PingRequest>::new().unwrap();
    mainloop.add_channel(ch, State { counter: 0 }).unwrap();

    let mut buffer = MessageBuffer::new();
    loop {
        let ev = mainloop.next().unwrap();
        match ev {
            Event::Message { ch, state, m } => {
                println!("[pong] received message: {}", m.int_value1());
                state.counter += 1;

                let reply = PingReply { int_value2: state.counter };
                if let Err(err) = ch.send_with_buffer(&mut buffer, reply) {
                    println!("failed to reply: {:?}", err);
                }
            }
        }
    }
}
