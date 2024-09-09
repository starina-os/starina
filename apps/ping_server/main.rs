#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_autogen::idl::ping::PingReply;
use ftl_autogen::idl::Message;

#[derive(Debug)]
enum Context {
    Startup,
    Client { counter: i32 },
}

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("ping_server started");
    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
    let startup_ch = env.take_channel("dep:startup").unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();

    loop {
        match mainloop.next() {
            Event::Message(Context::Startup, Message::NewClient(m), _sender) => {
                let new_ch = m.handle.take::<Channel>().unwrap();
                mainloop
                    .add_channel(new_ch, Context::Client { counter: 0 })
                    .unwrap();
            }
            Event::Message(Context::Client { counter }, Message::Ping(_m), sender) => {
                let reply = PingReply { value: *counter };
                *counter += 1;

                if let Err(err) = sender.send(reply) {
                    warn!("failed to reply: {:?}", err);
                }
            }
            ev => {
                panic!("unexpected event: {:?}", ev);
            }
        }
    }
}
