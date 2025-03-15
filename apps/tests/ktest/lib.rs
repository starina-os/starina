#![no_std]

pub mod autogen;

use starina::app::Context;
use starina::app::Dispatcher;
use starina::app::Mainloop;
use starina::channel::Channel;
use starina::info;
use starina::message::Message;
use starina::message::Open;

pub struct App {}

impl Mainloop for App {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self {
        info!("Hello from ktest!");
        ch.send(Open { uri: "ping" }).unwrap();
        dispatcher.add_channel(ch).unwrap();
        info!("added channel");
        App {}
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        info!("ktest: ch={:?}: open={}", ctx.sender.handle(), msg.uri());
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
