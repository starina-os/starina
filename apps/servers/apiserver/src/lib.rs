#![no_std]

mod endpoints;
mod http;

use http::RequestParser;
use http::TryFlushResult;
use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
use starina::channel::RecvError;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::CallId;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::sync::Mutex;

use crate::http::BufferedResponseWriter;
use crate::http::HeaderName;
use crate::http::ResponseWriter;
use crate::http::StatusCode;

pub const SPEC: AppSpec = AppSpec {
    name: "apiserver",
    env: &[EnvItem {
        name: "tcpip",
        ty: EnvType::Service { service: "tcpip" },
    }],
    exports: &[],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub tcpip: Channel,
}

struct Client {
    parser: RequestParser,
    resp: BufferedResponseWriter,
}

enum State {
    Tcpip(ChannelReceiver),
    Listen(Channel),
    Data {
        client: Mutex<Client>,
        ch: ChannelReceiver,
    },
}

fn main(env_json: &[u8]) {
    let env: Env = serde_json::from_slice(env_json).expect("Failed to parse environment");

    let mut msgbuffer = MessageBuffer::new();
    let poll = Poll::new().unwrap();
    let (tcpip_tx, tcpip_rx) = env.tcpip.split();

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
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::OpenReply { call_id, handle }) => {
                        assert_eq!(call_id, open_call_id);
                        break 'initloop handle;
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on tcpip channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!(
                            "unhandled message type on tcpip channel: {}",
                            msginfo.kind()
                        );
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on tcpip channel: {:?}", err);
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

    info!("API server listening on port 8080");
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
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Connect { handle }) => {
                        let handle_id = handle.handle_id();
                        info!("new client connection with handle {:?}", handle_id);

                        let (sender, receiver) = handle.split();
                        let mut resp = BufferedResponseWriter::new(sender);
                        resp.headers_mut()
                            .insert(HeaderName::SERVER, "Starina/apiserver")
                            .unwrap();

                        poll.add(
                            handle_id,
                            State::Data {
                                client: Mutex::new(Client {
                                    parser: RequestParser::new(),
                                    resp,
                                }),
                                ch: receiver,
                            },
                            Readiness::READABLE | Readiness::CLOSED,
                        )
                        .unwrap();
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on listen channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!(
                            "unhandled message type on listen channel: {}",
                            msginfo.kind()
                        );
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on listen channel: {:?}", err);
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
            State::Data { ch, client } if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Data { data }) => {
                        let mut client_guard = client.lock();
                        let client = &mut *client_guard;

                        // Parse the request, and route it.
                        let result = match client.parser.parse_chunk(data) {
                            Ok(Some(request)) => endpoints::route(&request, &mut client.resp),
                            Ok(None) => {
                                trace!("Partial HTTP request received, waiting for more data");
                                continue;
                            }
                            Err(e) => Err(anyhow::format_err!("HTTP parsing error: {:?}", e)),
                        };

                        // Write an error response.
                        let resp = &mut client.resp;
                        if let Err(e) = result {
                            if resp.are_headers_sent() {
                                // It's too late to send an error response.
                                debug_warn!(
                                    "HTTP error response already sent, cannot send error: {}",
                                    e
                                );
                                return;
                            }

                            let headers = resp.headers_mut();
                            headers
                                .insert(HeaderName::CONTENT_TYPE, "text/plain")
                                .unwrap();

                            resp.write_headers(StatusCode::INTERNAL_SERVER_ERROR);
                            resp.write_body(e.to_string().as_bytes());
                        }

                        // Flush the response.
                        match resp.try_flush() {
                            TryFlushResult::Done => {
                                debug!(
                                    "API server: response fully flushed, waiting for TCP to close"
                                );
                                poll.remove(ch.handle_id()).unwrap();
                            }
                            TryFlushResult::Partial => {
                                // Still need to flush more, will wait for WRITABLE events.
                                poll.listen(ch.handle_id(), Readiness::WRITABLE).unwrap();
                            }
                            TryFlushResult::Error(e) => {
                                debug_warn!("Failed to flush response: {:?}", e);
                                poll.remove(ch.handle_id()).unwrap();
                            }
                        }
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on data channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!("unhandled message type on data channel: {}", msginfo.kind());
                    }
                    Err(RecvError::Syscall(ErrorCode::WouldBlock)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on data channel: {:?}", err);
                    }
                }
            }
            State::Data { ch, client } if readiness.contains(Readiness::WRITABLE) => {
                debug!("WRITABLE event for handle {:?}", ch.handle_id());
                let mut client = client.lock();
                match client.resp.try_flush() {
                    TryFlushResult::Done => {
                        debug!("API server: response fully flushed, waiting for TCP to close");
                        poll.remove(ch.handle_id()).unwrap();
                    }
                    TryFlushResult::Partial => {
                        trace!(
                            "WRITABLE event for handle {:?}, still need to flush more",
                            ch.handle_id()
                        );
                    }
                    TryFlushResult::Error(e) => {
                        debug_warn!("Failed to flush response: {:?}", e);
                        poll.remove(ch.handle_id()).unwrap();
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
        }
    }
}
