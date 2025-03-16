#![no_std]

pub mod autogen;

use starina::channel::Channel;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::info;
use starina::message::Message;
use starina::message::Open;

pub struct App {}

impl EventLoop for App {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self {
        info!("Hello from virtio-net!");
        App {}
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
