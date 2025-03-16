#![no_std]

pub mod autogen;

use autogen::Env;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::info;
use starina::message::Message;
use starina::message::Open;

pub struct App {}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        info!("Hello from virtio-net!");
        info!("device_tree: {:?}", env.device_tree);
        App {}
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
