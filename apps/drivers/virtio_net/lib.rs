#![no_std]

pub mod autogen;

use autogen::Env;
use starina::channel::ChannelSender;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::interrupt::Interrupt;
use starina::message::ConnectMsg;
use starina::message::FramedDataMsg;
use starina::prelude::*;
use virtio_net::VirtioNet;

mod virtio_net;

struct ReceiverImpl(ChannelSender);

impl virtio_net::Receiver for ReceiverImpl {
    fn receive(&mut self, data: &[u8]) {
        if let Err(err) = self.0.send(FramedDataMsg { data }) {
            debug_warn!("failed to forward a packet to upstream: {:?}", err);
        }
    }
}

pub enum State {
    Startup,
    Interrupt,
    Upstream,
}

pub struct App {
    virtio_net: spin::Mutex<VirtioNet>,
}

impl EventLoop<Env> for App {
    type State = State;

    fn init(dispatcher: &Dispatcher<Self::State>, mut env: Env) -> Self {
        dispatcher
            .add_channel(State::Startup, env.startup_ch)
            .unwrap();

        let mut virtio_net = VirtioNet::init_or_panic(&env.device_tree, &mut env.iobus);
        let interrupt = virtio_net.take_interrupt().unwrap();
        dispatcher
            .add_interrupt(State::Interrupt, interrupt)
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

    fn on_connect(&self, ctx: Context<Self::State>, msg: ConnectMsg) {
        let upstream = ctx
            .dispatcher
            .add_channel(State::Upstream, msg.handle)
            .unwrap();

        self.virtio_net.lock().set_receiver(ReceiverImpl(upstream));
    }

    fn on_framed_data(&self, _ctx: Context<Self::State>, msg: FramedDataMsg<'_>) {
        trace!("frame data received: {} bytes", msg.data.len());
        self.virtio_net.lock().transmit(msg.data);
    }

    fn on_interrupt(&self, interrupt: &Interrupt) {
        interrupt.acknowledge().unwrap();
        self.virtio_net.lock().handle_interrupt();
    }
}
