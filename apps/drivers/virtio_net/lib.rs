#![no_std]

pub mod autogen;

use autogen::Env;
use starina::channel::Channel;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::interrupt::Interrupt;
use starina::message::Connect;
use starina::message::FramedData;
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
        let interrupt = virtio_net.take_interrupt().unwrap();
        dispatcher
            .add_interrupt(interrupt)
            .expect("failed to add interrupt");

        // Update the source mac address.
        let mac = virtio_net.mac_addr();
        debug!(
            "MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        );

        Self {
            virtio_net: spin::Mutex::new(virtio_net),
        }
    }

    fn on_connect(&self, ctx: &Context, mut msg: Message<Connect>) {
        let handle = Channel::from_handle(msg.handle().unwrap()); // FIXME: type check?
        ctx.dispatcher.add_channel(handle).unwrap();
    }

    fn on_framed_data(&self, _ctx: &Context, msg: Message<FramedData<'_>>) {
        trace!("frame data received: {:2x?}", msg.data());
    }

    fn on_interrupt(&self, interrupt: &Interrupt) {
        interrupt.acknowledge().unwrap();
        self.virtio_net.lock().handle_interrupt(|pkt| {
            info!("packet received: {:02x?}", pkt);
        });
    }
}
