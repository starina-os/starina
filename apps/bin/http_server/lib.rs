#![no_std]

pub mod autogen;
mod connection;
mod http;

use autogen::Env;
use connection::ChannelWriter;
use connection::Conn;
use starina::collections::HashMap;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::handle::HandleId;
use starina::handle::Handleable;
use starina::message::ConnectMsg;
use starina::message::OpenMsg;
use starina::message::OpenReplyMsg;
use starina::message::StreamDataMsg;
use starina::prelude::*;

#[derive(Debug)]
enum CtrlState {
    Opening,
    Ready,
}

pub struct App {
    state: spin::Mutex<CtrlState>,
    connections: spin::Mutex<HashMap<HandleId, Conn<ChannelWriter>>>,
}

impl EventLoop<Env> for App {
    fn init(dispatcher: &Dispatcher, env: Env) -> Self {
        let tcpip = env.tcpip;

        // let uri = format!("tcp:{}:{}", env.listen_host, env.listen_port);
        info!("connecting to tcpip");
        let uri = format!("tcp-listen:0.0.0.0:80");
        tcpip.send(OpenMsg { uri: &uri }).unwrap();

        dispatcher.split_and_add_channel(tcpip).unwrap();
        Self {
            state: spin::Mutex::new(CtrlState::Opening),
            connections: spin::Mutex::new(HashMap::new()),
        }
    }

    fn on_open_reply(&self, ctx: &Context, msg: OpenReplyMsg) {
        info!("got open-reply");

        // FIXME: Check txid
        let listen_ch = msg.handle;
        ctx.dispatcher.add_channel(listen_ch).unwrap();

        let mut state = self.state.lock();
        assert!(matches!(*state, CtrlState::Opening));
        *state = CtrlState::Ready;
    }

    fn on_connect(&self, ctx: &Context, msg: ConnectMsg) {
        info!("got connect");
        // FIXME: Check sender channel - it must be the listen channel
        let data_ch_id = msg.handle.handle_id();
        let sender = ctx
            .dispatcher
            .split_and_add_channel(msg.handle)
            .expect("failed to get channel sender");
        let mut connections = self.connections.lock();
        let tcp_writer = ChannelWriter::new(sender);
        connections.insert(data_ch_id, Conn::new(tcp_writer));
    }

    fn on_stream_data(&self, ctx: &Context, msg: StreamDataMsg<'_>) {
        info!("got stream data");
        let mut connections = self.connections.lock();
        let Some(conn) = connections.get_mut(&ctx.sender.handle().id()) else {
            debug_warn!(
                "stream data from an unexpected channel: {:?}",
                ctx.sender.handle().id()
            );
            return;
        };

        conn.on_tcp_data(msg.data);
    }
}
