#![no_std]

use core::ops::ControlFlow;

use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::device_tree::DeviceTree;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::interrupt::Interrupt;
use starina::message::FramedDataMsg;
use starina::message::Message2;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::ParsedAppSpec;
use starina::spec::ParsedDeviceMatch;
use starina::spec::ParsedEnvItem;
use starina::spec::ParsedEnvType;
use starina::spec::ParsedExportItem;
use virtio_net::VirtioNet;

mod virtio_net;

pub enum State {
    Startup(Channel),
    Interrupt(Interrupt),
    Upstream(ChannelReceiver),
}

#[derive(Debug, Deserialize)]
pub struct Env {
    pub startup_ch: Channel,
    pub device_tree: DeviceTree,
}

pub const APP_SPEC: ParsedAppSpec = ParsedAppSpec {
    name: "virtio_net",
    env: &[ParsedEnvItem {
        name: "device_tree",
        ty: ParsedEnvType::DeviceTree {
            matches: &[ParsedDeviceMatch::Compatible("virtio,mmio")],
        },
    }],
    exports: &[ParsedExportItem::Service {
        service: "device/ethernet",
    }],
    main,
};

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
        Readiness::READABLE,
    )
    .unwrap();

    // Start watching for interrupts.
    let interrupt = virtio_net.take_interrupt().unwrap();
    poll.add(
        interrupt.handle_id(),
        State::Interrupt(interrupt),
        Readiness::READABLE,
    )
    .unwrap();

    let mut upstream_sender = None;
    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) => {
                info!("startup: connected to channel");
                if readiness.contains(Readiness::READABLE) {
                    let mut m = ch.recv().unwrap();
                    let Some(m) = m.parse() else {
                        debug_warn!("failed to parse a message, ignoring");
                        continue;
                    };

                    match m {
                        Message2::Connect(m) => {
                            let (sender, receiver) = m.handle.split();
                            upstream_sender = Some(sender);
                            poll.add(
                                receiver.handle().id(),
                                State::Upstream(receiver),
                                Readiness::READABLE,
                            )
                            .unwrap();
                        }
                        _ => {
                            debug_warn!("unhandled message");
                        }
                    }
                }
            }
            State::Upstream(ch) => {
                if readiness.contains(Readiness::READABLE) {
                    let mut m = ch.recv().unwrap();
                    let Some(m) = m.parse() else {
                        debug_warn!("failed to parse a message, ignoring");
                        continue;
                    };

                    match m {
                        Message2::FramedData(m) => {
                            trace!("transmitting {} bytes", m.data.len());
                            virtio_net.transmit(m.data);
                        }
                        _ => {
                            debug_warn!("unhandled message");
                        }
                    }
                }
            }
            State::Interrupt(interrupt) => {
                info!("interrupt: received interrupt");
                if readiness.contains(Readiness::READABLE) {
                    interrupt.acknowledge().unwrap();
                    virtio_net.handle_interrupt(|data| {
                        if let Some(sender) = upstream_sender.as_ref() {
                            match sender.send(FramedDataMsg { data }) {
                                Ok(()) => {}
                                Err(ErrorCode::Full) => {
                                    debug_warn!("backpressure: upstream channel is full");
                                    return ControlFlow::Break(());
                                }
                                Err(err) => {
                                    error!("failed to forward a packet to upstream: {:?}", err);
                                }
                            }
                        }

                        ControlFlow::Continue(())
                    });
                }
            }
        }
    }
}
