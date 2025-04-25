#![no_std]

pub mod autogen;
mod connection;
mod http;

use connection::ChannelWriter;
use connection::Conn;
use starina::eventloop::Context;
use starina::eventloop::Dispatcher;
use starina::eventloop::EventLoop;
use starina::message::CallId;
use starina::message::ConnectMsg;
use starina::message::OpenMsg;
use starina::message::OpenReplyMsg;
use starina::message::StreamDataMsg;
use starina::prelude::*;
use starina::sync::Mutex;

#[derive(Debug)]
enum AppState {
    Opening,
    Ready,
}

#[derive(Debug)]
pub enum State {
    Startup,
    Tcpip,
    Listen,
    Data { conn: Conn },
}

pub struct App {
    state: Mutex<AppState>,
}

impl EventLoop for App {
    type Env = autogen::Env;
    type State = State;

    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self {
        let tcpip = env.tcpip;

        // let uri = format!("tcp:{}:{}", env.listen_host, env.listen_port);
        info!("connecting to tcpip");
        let uri = b"tcp-listen:0.0.0.0:80";

        let call_id = CallId::from(1);
        tcpip.call(call_id, OpenMsg { uri }).unwrap();

        dispatcher.add_channel(State::Tcpip, tcpip).unwrap();
        Self {
            state: Mutex::new(AppState::Opening),
        }
    }

    fn on_open_reply(&self, ctx: Context<Self::State>, call_id: CallId, msg: OpenReplyMsg) {
        assert_eq!(call_id, CallId::from(1));

        info!("got open-reply");

        // FIXME: Check txid
        let listen_ch = msg.handle;
        ctx.dispatcher
            .add_channel(State::Listen, listen_ch)
            .unwrap();

        let mut state = self.state.lock();
        assert!(matches!(*state, AppState::Opening));
        *state = AppState::Ready;
    }

    fn on_connect(&self, ctx: Context<Self::State>, msg: ConnectMsg) {
        if !matches!(ctx.state, State::Listen) {
            debug_warn!("connect message from unexpected state: {:?}", ctx.state);
            return;
        }

        trace!("new client connection");
        let conn = Conn::new();
        ctx.dispatcher
            .add_channel(State::Data { conn }, msg.handle)
            .unwrap();
    }

    fn on_stream_data(&self, ctx: Context<Self::State>, msg: StreamDataMsg<'_>) {
        let State::Data { conn } = ctx.state else {
            debug_warn!("stream data from unexpected state: {:?}", ctx.state);
            return;
        };

        let writer = ChannelWriter::new(ctx.sender);
        conn.on_tcp_data(writer, msg.data);
    }
}
