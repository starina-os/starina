#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_api::types::idl::StringField;
use ftl_api_autogen::protocols::ping::PingReply;
use ftl_autogen2_generated::Message;

#[derive(Debug)]
enum Context {
    Startup,
    Client { counter: i32 },
}

#[ftl_api::main]
pub fn main(mut env: Environ) {
    info!("start main...");

    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    let startup_ch = env.take_channel("dep:startup").unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();

    loop {
        match mainloop.next() {
            Event::Message(Context::Startup, Message::NewclientRequest(mut m), _sender) => {
                let new_ch = m.handle().unwrap();
                mainloop
                    .add_channel(new_ch, Context::Client { counter: 0 })
                    .unwrap();
            }
            Event::Message(Context::Client { counter }, Message::PingRequest(m), sender) => {
                info!(
                    "received message: {} {:02x?}",
                    m.int_value1(),
                    m.bytes_value1().as_slice()
                );
                *counter += 1;

                let reply = PingReply {
                    int_value2: *counter,
                    str_value2: StringField::try_from("howdy!").unwrap(),
                };
                if let Err(err) = sender.send(reply) {
                    info!("failed to reply: {:?}", err);
                }
            }
            ev => {
                panic!("unexpected event: {:?}", ev);
            }
        }
    }
}
