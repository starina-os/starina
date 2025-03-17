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
        for (name, node) in env.device_tree.devices {
            if !node.compatible.iter().any(|c| c == "virtio,mmio") {
                continue;
            }

            info!("device: {}", name);
            info!("  reg: {:x?}", node.reg);
        }
        App {}
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
