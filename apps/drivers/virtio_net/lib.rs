#![no_std]

pub mod autogen;

use autogen::Env;
use starina::address::DAddr;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::info;
use starina::message::Message;
use starina::message::Open;
use virtio::transports::VirtioTransport;
use virtio::transports::mmio::VirtioMmio;

fn probe(env: Env) -> Option<VirtioMmio> {
    for (name, node) in env.device_tree.devices {
        if !node.compatible.iter().any(|c| c == "virtio,mmio") {
            continue;
        }

        info!("device: {}", name);
        info!("  reg: {:x?}", node.reg);

        let folio = MmioFolio::create(&iobus, DAddr::new(node.reg[0]), node.reg[0].len).unwrap();
        let virtio = VirtioMmio::new(folio);
        let device_type = virtio.probe();
        if device_type == Some(DeviceType::Net) {
            return Some(virtio);
        }
    }

    None
}

pub struct App {
    virtio: VirtioMmio,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        let virtio = probe(env).expect("failed to probe virtio-net device");
        App { virtio }
    }

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
