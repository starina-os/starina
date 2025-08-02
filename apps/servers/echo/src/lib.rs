#![no_std]

use serde::Deserialize;
use starina::channel::Channel;
use starina::mainloop::ChannelContext;
use starina::mainloop::ChannelHandler;
use starina::mainloop::StartupContext;
use starina::mainloop::StartupHandler;
use starina::message::Message;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::ExportItem;

pub const SPEC: AppSpec = AppSpec {
    name: "echo",
    env: &[],
    exports: &[ExportItem::Service { service: "echo" }],
    main: starina::mainloop::run::<App, Env>,
};

#[derive(Deserialize)]
struct Env {}

struct App {}

impl StartupHandler<Env> for App {
    fn init(_ctx: &StartupContext, _env: Env) -> Self {
        info!("starting echo server");
        Self {}
    }

    fn connected(&self, ctx: &StartupContext, ch: Channel) {
        info!("client connected");
        ctx.dispatcher.add_channel(ch, DataChannel {}).unwrap();
    }
}

struct DataChannel {}

impl ChannelHandler for DataChannel {
    fn data(&self, ctx: &ChannelContext, data: &[u8]) {
        ctx.sender.send(Message::Data { data }).unwrap();
    }
}
