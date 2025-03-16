#![no_std]

pub mod autogen;

use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::info;
use starina::message::Message;
use starina::message::Open;

pub struct App {}

impl EventLoop<()> for App {
    fn init(dispatcher: &Dispatcher, _: ()) -> Self {
        info!("Hello from ktest!");
        App {}
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        info!("ktest: ch={:?}: open={}", ctx.sender.handle(), msg.uri());
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
