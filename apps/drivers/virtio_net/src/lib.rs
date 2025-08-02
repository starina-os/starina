#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelSender;
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

struct Mutable {
    virtio_net: VirtioNet,
    upstream_sender: Option<ChannelSender>,
}

struct App {
    mutable: Arc<Mutex<Mutable>>,
}

struct UpstreamState {
    mutable: Arc<Mutex<Mutable>>,
}

struct InterruptState {
    mutable: Arc<Mutex<Mutable>>,
}

impl StartupHandler<Env> for App {
    fn init(ctx: &StartupContext, env: Env) -> Self {
        let mut virtio_net = VirtioNet::init_or_panic(&env.device_tree);

        let mac = virtio_net.mac_addr();
        debug!(
            "MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        );

        let interrupt = virtio_net.take_interrupt().unwrap();

        let mutable = Arc::new(Mutex::new(Mutable {
            virtio_net,
            upstream_sender: None,
        }));

        let interrupt_handler = InterruptState {
            mutable: mutable.clone(),
        };
        ctx.dispatcher
            .add_interrupt(interrupt, interrupt_handler)
            .unwrap();

        Self { mutable }
    }

    fn connected(&self, ctx: &StartupContext, ch: Channel) {
        info!("upstream connected");
        let (sender, receiver) = ch.split();
        self.mutable.lock().upstream_sender = Some(sender.clone());
        ctx.dispatcher
            .add_channel(
                (sender, receiver),
                UpstreamState {
                    mutable: self.mutable.clone(),
                },
            )
            .unwrap();
    }
}

impl ChannelHandler for UpstreamState {
    fn data(&self, _ctx: &ChannelContext, data: &[u8]) {
        trace!("transmitting {} bytes", data.len());
        let mut mutable = self.mutable.lock();
        mutable.virtio_net.transmit(data);
    }

    fn disconnected(&self, _ctx: &ChannelContext) {
        warn!("upstream channel disconnected");
        let mut mutable = self.mutable.lock();
        mutable.upstream_sender = None;
    }
}

impl InterruptHandler for InterruptState {
    fn interrupt(&self, _ctx: &InterruptContext) {
        trace!("interrupt: received interrupt");
        let mut mutable = self.mutable.lock();
        let upstream_sender = mutable.upstream_sender.clone();
        mutable.virtio_net.handle_interrupt(|data| {
            let Some(sender) = upstream_sender.as_ref() else {
                debug_warn!("upstream channel is not connected, dropping packet");
                return;
            };

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
