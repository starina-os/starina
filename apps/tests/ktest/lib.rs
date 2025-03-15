#![no_std]

pub mod autogen;

use starina::app::Context;
use starina::app::Dispatcher;
use starina::app::Mainloop;
use starina::channel::AnyMessage;
use starina::channel::Channel;
use starina::channel::message::PingReader;
use starina::channel::message::PingWriter;
use starina::info;

pub struct App {}

impl Mainloop for App {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self {
        info!("Hello from ktest!");
        ch.send(PingWriter { value: 0 }).unwrap();
        dispatcher.add_channel(ch).unwrap();
        info!("added channel");
        App {}
    }

    fn on_message(&self, ctx: &Context, msg: AnyMessage) {
        let ping = PingReader::try_from(msg).unwrap();
        info!("ktest: ch={:?}: ping={}", ctx.sender.handle(), ping.value());
        ctx.sender
            .send(PingWriter {
                value: ping.value() + 1,
            })
            .unwrap();
    }
}
