#![no_std]

pub mod autogen;

use autogen::Env;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::message::Message;
use starina::message::Open;
use starina::prelude::*;
use virtio_net::VirtioNet;

mod virtio_net;

pub struct App {
    virtio_net: VirtioNet,
}

impl EventLoop<Env> for App {
    fn init(_dispatcher: &Dispatcher, env: Env) -> Self {
        let mut virtio_net = VirtioNet::init_or_panic(env);
        info!("submitting arp request");
        virtio_net.transmit(&[
            // Ethernet header
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // Destination MAC (broadcast)
            0x52, 0x54, 0x00, 0x12, 0x34, 0x56, // Source MAC (example)
            0x08, 0x06, // EtherType (ARP)
            // ARP packet
            0x00, 0x01, // Hardware type (Ethernet)
            0x08, 0x00, // Protocol type (IPv4)
            0x06, // Hardware address length (6 bytes for MAC)
            0x04, // Protocol address length (4 bytes for IPv4)
            0x00, 0x01, // Operation (1 = request)
            0x52, 0x54, 0x00, 0x12, 0x34, 0x56, // Sender MAC
            192, 168, 1, 10, // Sender IP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Target MAC (zeros for request)
            192, 168, 1, 1, // Target IP
        ]);
        Self { virtio_net }
    }

    fn on_open(&self, ctx: &Context, _msg: Message<Open<'_>>) {
        ctx.sender.send(Open { uri: "pong" }).unwrap();
    }
}
