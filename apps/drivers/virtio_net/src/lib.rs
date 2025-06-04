#![no_std]

use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::device_tree::DeviceTree;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::interrupt::Interrupt;
use starina::message::FramedDataMsg;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::DeviceMatch;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use virtio_net::VirtioNet;

mod virtio_net;

pub const SPEC: AppSpec = AppSpec {
    name: "virtio_net",
    env: &[EnvItem {
        name: "device_tree",
        ty: EnvType::DeviceTree {
            matches: &[DeviceMatch::Compatible("virtio,mmio")],
        },
    }],
    exports: &[ExportItem::Service {
        service: "device/ethernet",
    }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub startup_ch: Channel,
    pub device_tree: DeviceTree,
}

enum State {
    Startup(Channel),
    Interrupt(Interrupt),
    Upstream(ChannelReceiver),
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).expect("failed to deserialize env");

    // Look for and initialize the virtio-net device.
    let mut virtio_net = VirtioNet::init_or_panic(&env.device_tree);

    // Read its MAC address.
    let mac = virtio_net.mac_addr();
    debug!(
        "MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
    );

    let poll = Poll::new().unwrap();
    poll.add(
        env.startup_ch.handle_id(),
        State::Startup(env.startup_ch),
        Readiness::READABLE | Readiness::CLOSED, /* FIXME: This should guarantee level-triggered */
    )
    .unwrap();

    // Start watching for interrupts.
    let interrupt = virtio_net.take_interrupt().unwrap();
    poll.add(
        interrupt.handle_id(),
        State::Interrupt(interrupt),
        Readiness::READABLE | Readiness::CLOSED, /* FIXME: This should guarantee level-triggered */
    )
    .unwrap();

    let mut upstream_sender = None;
    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::Connect { handle }) => {
                        let (sender, receiver) = handle.split();
                        upstream_sender = Some(sender);
                        poll.add(
                            receiver.handle().id(),
                            State::Upstream(receiver),
                            Readiness::READABLE | Readiness::CLOSED, /* FIXME: This should guarantee level-triggered */
                        )
                        .unwrap();
                    }
                    _ => {
                        debug_warn!("unhandled message: {:?}", m.msginfo);
                    }
                }
            }
            State::Startup(_) => {
                panic!("unexpected readiness for startup channel: {:?}", readiness);
            }
            State::Upstream(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::FramedData { data }) => {
                        trace!("transmitting {} bytes", data.len());
                        virtio_net.transmit(data);
                    }
                    _ => {
                        debug_warn!("unhandled message {:?}", m.msginfo);
                    }
                }
            }
            State::Upstream(ch) if readiness == Readiness::CLOSED => {
                warn!("upstream channel closed, stopping transmission");
                poll.remove(ch.handle().id()).unwrap();
                upstream_sender = None;
            }
            &State::Upstream(_) => {
                panic!("unexpected readiness for upstream channel: {:?}", readiness);
            }
            State::Interrupt(interrupt) if readiness.contains(Readiness::READABLE) => {
                trace!("interrupt: received interrupt");
                interrupt.acknowledge().unwrap();
                virtio_net.handle_interrupt(|data| {
                    let Some(sender) = upstream_sender.as_ref() else {
                        debug_warn!("upstream channel is not connected, dropping packet");
                        return;
                    };

                    if let Err(err) = sender.send(FramedDataMsg { data }) {
                        if err == ErrorCode::Full {
                            // We don't backpressure the virtqueue because both the upstream
                            // and the peer over the network should retry later.
                            debug_warn!("upstream channel is full, dropping packet");
                        } else {
                            error!("failed to send packet upstream: {:?}", err);
                        }
                    }
                });
            }
            State::Interrupt(_) => {
                panic!("unexpected readiness for interrupt: {:?}", readiness);
            }
        }
    }
}
