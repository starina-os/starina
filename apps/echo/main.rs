#![no_std]
#![no_main]

starina_api::autogen!();

use starina_api::channel::Channel;
use starina_api::environ::Environ;
use starina_api::mainloop::Event;
use starina_api::mainloop::Mainloop;
use starina_api::prelude::*;
use starina_autogen::idl::echo::PingReply;
use starina_autogen::idl::Message;

#[derive(Debug)]
enum Context {
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
                ..
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
