#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use serde::Deserialize;
use starina::channel::Channel;
use starina::device_tree::DeviceTree;
use starina::mainloop::ChannelContext;
use starina::mainloop::ChannelHandler;
use starina::mainloop::InterruptContext;
use starina::mainloop::InterruptHandler;
use starina::mainloop::StartupContext;
use starina::mainloop::StartupHandler;
use starina::message::Message;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::DeviceMatch;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::sync::Mutex;
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
    main: starina::mainloop::run::<App, Env>,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub device_tree: DeviceTree,
}

struct App {
    virtio_net: Arc<Mutex<VirtioNet>>,
}

struct UpstreamState(Arc<Mutex<VirtioNet>>);
struct InterruptState(Arc<Mutex<VirtioNet>>);

impl StartupHandler<Env> for App {
    fn init(ctx: &StartupContext, env: Env) -> Self {
        let mut virtio_net = VirtioNet::init_or_panic(&env.device_tree);
        let interrupt = virtio_net.take_interrupt().unwrap();
        let virtio_net = Arc::new(Mutex::new(virtio_net));
        ctx.dispatcher
            .add_interrupt(interrupt, InterruptState(virtio_net.clone()))
            .unwrap();

        Self { virtio_net }
    }

    fn connected(&self, ctx: &StartupContext, ch: Channel) {
        info!("upstream connected");
        let (sender, receiver) = ch.split();
        ctx.dispatcher
            .add_channel(
                (sender.clone(), receiver),
                UpstreamState(self.virtio_net.clone()),
            )
            .unwrap();

        self.virtio_net.lock().set_receive_callback(move |data| {
            if let Err(err) = sender.send(Message::Data { data }) {
                if err == starina::error::ErrorCode::Full {
                    debug_warn!("upstream channel is full, dropping packet");
                } else {
                    error!("failed to send packet upstream: {:?}", err);
                }
            }
        });
    }
}

impl ChannelHandler for UpstreamState {
    fn data(&self, _ctx: &ChannelContext, data: &[u8]) {
        self.0.lock().transmit(data);
    }

    fn disconnected(&self, _ctx: &ChannelContext) {
        self.0.lock().set_receive_callback(|_data| {
            // No upstream. Drop received packets.
        });
    }
}

impl InterruptHandler for InterruptState {
    fn interrupt(&self, _ctx: &InterruptContext) {
        self.0.lock().handle_interrupt();
    }
}
