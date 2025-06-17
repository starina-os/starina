#![no_std]

mod connection;
mod http;

use connection::ChannelWriter;
use connection::Conn;
use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::Message;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::spec::ExportItem;
use starina::sync::Mutex;

pub const SPEC: AppSpec = AppSpec {
    name: "http_server",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[ExportItem::Service {
        service: "http_server",
    }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub startup_ch: Channel,
    pub tcpip: Channel,
}

enum State {
    Startup,
    Tcpip(ChannelReceiver),
    Listen(Channel),
    Data { ch: Channel, conn: Mutex<Conn> },
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).expect("Failed to parse environment");

    let poll = Poll::new().unwrap();
    let (tcpip_tx, tcpip_rx) = env.tcpip.split();
    poll.add(
        env.startup_ch.handle_id(),
        State::Startup,
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();
    poll.add(
        tcpip_rx.handle_id(),
        State::Tcpip(tcpip_rx),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    let open_call_id = CallId::from(1);
    let uri = b"tcp-listen:0.0.0.0:80";
    tcpip_tx
        .send(Message::Open {
            call_id: open_call_id,
            uri,
        })
        .unwrap();

    let listen_ch = 'initloop: loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Tcpip(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::OpenReply { call_id, handle }) => {
                        assert_eq!(call_id, open_call_id);
                        break 'initloop handle;
                    }
                    _ => {
                        debug_warn!(
                            "unexpected message in tcpip channel while initializing: {:?}",
                            m.msginfo
                        );
                    }
                }
            }
            _ => {
                panic!(
                    "unexpected readiness during initialization: {:?}",
                    readiness
                );
            }
        }
    };

    info!("got listen channel: {:?}", listen_ch);
    poll.add(
        listen_ch.handle_id(),
        State::Listen(listen_ch),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Listen(ch) if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::Connect { handle }) => {
                        info!("new client connection");
                        let conn = Conn::new();
                        poll.add(
                            handle.handle_id(),
                            State::Data {
                                ch: handle,
                                conn: Mutex::new(conn),
                            },
                            Readiness::READABLE | Readiness::CLOSED,
                        )
                        .unwrap();
                    }
                    _ => {
                        debug_warn!("unexpected message in listen channel: {:?}", m.msginfo);
                    }
                }
            }
            State::Listen(ch) if readiness == Readiness::CLOSED => {
                warn!("listen channel closed, server shutting down");
                poll.remove(ch.handle_id()).unwrap();
                break;
            }
            State::Listen(_) => {
                panic!("unexpected readiness for listen channel: {:?}", readiness);
            }
            State::Data { conn, ch } if readiness.contains(Readiness::READABLE) => {
                let mut m = ch.recv().unwrap();
                match m.parse() {
                    Some(Message::StreamData { data }) => {
                        let writer = ChannelWriter::new(&ch);
                        conn.lock().on_tcp_data(writer, data);
                    }
                    _ => {
                        debug_warn!("unexpected message: {:?} in data channel", m.msginfo);
                    }
                }
            }
            State::Data { ch, .. } if readiness == Readiness::CLOSED => {
                trace!("data channel closed");
                poll.remove(ch.handle_id()).unwrap();
            }
            State::Data { .. } => {
                panic!("unexpected readiness for data channel: {:?}", readiness);
            }
            State::Tcpip(_) => {
                debug_warn!("unexpected readiness for tcpip channel: {:?}", readiness);
            }
            State::Startup => {
                debug_warn!("unexpected readiness for startup channel: {:?}", readiness);
            }
        }
    }
}

//     fn on_open_reply(&self, ctx: Context<Self::State>, call_id: CallId, msg: OpenReplyMsg) {
//         assert_eq!(call_id, CallId::from(1));

//         info!("got open-reply");

//         // FIXME: Check txid
//         let listen_ch = msg.handle;
//         ctx.dispatcher
//             .add_channel(State::Listen, listen_ch)
//             .unwrap();

//         let mut state = self.state.lock();
//         assert!(matches!(*state, AppState::Opening));
//         *state = AppState::Ready;
//     }

//     fn on_connect(&self, ctx: Context<Self::State>, msg: ConnectMsg) {
//         if !matches!(ctx.state, State::Listen) {
//             debug_warn!("connect message from unexpected state: {:?}", ctx.state);
//             return;
//         }

//         trace!("new client connection");
//         let conn = Conn::new();
//         ctx.dispatcher
//             .add_channel(State::Data { conn }, msg.handle)
//             .unwrap();
//     }

//     fn on_stream_data(&self, ctx: Context<Self::State>, msg: StreamDataMsg<'_>) {
//         let State::Data { conn } = ctx.state else {
//             debug_warn!("stream data from unexpected state: {:?}", ctx.state);
//             return;
//         };

//     }
// }
