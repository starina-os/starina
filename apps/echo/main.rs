#![no_std]
#![no_main]

ftl_api::autogen!();

use ftl_api::channel::Channel;
use ftl_api::environ::Environ;
use ftl_api::mainloop::Event;
use ftl_api::mainloop::Mainloop;
use ftl_api::prelude::*;
use ftl_autogen::idl::echo::PingReply;
use ftl_autogen::idl::Message;

#[derive(Debug)]
pub enum Context {
    Startup,
    Client,
}

#[no_mangle]
pub fn main(mut env: Environ) {
    let mut mainloop = Mainloop::<Context, Message>::new().unwrap();

    let startup_ch = env.take_channel("dep:startup").unwrap();
    mainloop.add_channel(startup_ch, Context::Startup).unwrap();

    info!("ready");
    loop {
        match mainloop.next() {
            Event::Message {
                ctx: Context::Startup,
                message: Message::NewClient(m),
                ..
            } => {
                let client_ch = m.handle.take::<Channel>().unwrap();
                mainloop.add_channel(client_ch, Context::Client).unwrap();
            }
            Event::Message {
                ctx: Context::Client,
                message: Message::Ping(m),
                sender,
            } => {
                let reply = PingReply { value: m.value };
                if let Err(err) = sender.send(reply) {
                    debug_warn!("failed to reply: {:?}", err);
                }
            }
            ev => {
                warn!("unhandled event: {:?}", ev);
            }
        }
    }
}
