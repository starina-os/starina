#![no_std]

use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::RecvError;
use starina::environ::Environ;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;
use starina::prelude::*;
use starina::spec::AppSpec;
use starina::spec::ExportItem;

pub const SPEC: AppSpec = AppSpec {
    name: "echo",
    env: &[],
    exports: &[ExportItem::Service { service: "echo" }],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub startup_ch: Channel,
}

enum State {
    Startup(Channel),
    Client(Channel),
}

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("Failed to parse environment");

    let poll = Poll::new().unwrap();

    poll.add(
        env.startup_ch.handle_id(),
        State::Startup(env.startup_ch),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    info!("started echo server");

    let mut msgbuffer = MessageBuffer::new();
    loop {
        let (state, readiness) = poll.wait().unwrap();
        match &*state {
            State::Startup(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Connect { ch }) => {
                        let handle_id = ch.handle_id();
                        info!("new client connection with handle {:?}", handle_id);

                        poll.add(
                            handle_id,
                            State::Client(ch),
                            Readiness::READABLE | Readiness::CLOSED,
                        )
                        .unwrap();
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on startup channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!("unhandled message type on startup: {}", msginfo.kind());
                    }
                    Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on startup: {:?}", err);
                    }
                }
            }
            State::Client(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Data { data }) => {
                        info!(
                            "received echo request: {:?}",
                            core::str::from_utf8(data).unwrap_or("<invalid utf8>")
                        );
                        ch.send(Message::Data { data }).unwrap();
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message on client channel: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!("unhandled message type on client: {}", msginfo.kind());
                    }
                    Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error on client: {:?}", err);
                    }
                }
            }
            State::Client(ch) if readiness == Readiness::CLOSED => {
                let handle_id = ch.handle_id();
                trace!("client disconnected: {:?}", handle_id);
                poll.remove(handle_id).unwrap();
            }
            State::Startup(_) if readiness == Readiness::CLOSED => {
                panic!("startup channel closed");
            }
            _ => {
                panic!("unexpected readiness: {:?}", readiness);
            }
        }
    }
}
