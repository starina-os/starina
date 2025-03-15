#![no_std]

pub mod autogen;

use starina::app::Context;
use starina::app::Dispatcher;
use starina::app::Mainloop;
use starina::channel::Channel;
use starina::info;
use starina::message::AnyMessage;
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

    fn on_message(&self, ctx: &Context, msg: AnyMessage) {
        let open = msg.as_open().unwrap();
        info!("ktest: ch={:?}: open={}", ctx.sender.handle(), open.uri());
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
