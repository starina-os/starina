#![no_std]

pub mod autogen;

use autogen::Env;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::interrupt::Interrupt;
use starina::message::Message;
use starina::message::Open;
use starina::prelude::*;
use virtio_net::VirtioNet;

mod virtio_net;

pub struct App {
    virtio_net: spin::Mutex<VirtioNet>,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        let mut virtio_net = VirtioNet::init_or_panic(env);
        info!("submitting arp request");
        virtio_net.transmit(&[
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x52, 0x55, 0x0a, 0x00, 0x02, 0x02, 0x08, 0x06,
            0x00, 0x01, 0x08, 0x00, 0x06, 0x04, 0x00, 0x01, 0x52, 0x55, 0x0a, 0x00, 0x02, 0x02,
            0x0a, 0x00, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x02, 0x0f,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);

        let interrupt = virtio_net.take_interrupt().unwrap();
        dispatcher
            .add_interrupt(interrupt)
            .expect("failed to add interrupt");
        Self {
            virtio_net: spin::Mutex::new(virtio_net),
        }
    }

    fn on_open(&self, ctx: &Context, _msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }

    fn on_interrupt(&self, interrupt: &Interrupt) {
        interrupt.acknowledge().unwrap();
        self.virtio_net.lock().handle_interrupt(|pkt| {
            info!("packet received: {:02x?}", pkt);
        });
    }
}
