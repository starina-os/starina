#![no_std]

extern crate alloc;

use alloc::format;
use core::time::Duration;

use serde::Deserialize;
use starina::channel::Channel;
use starina::channel::ChannelReceiver;
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
use starina::spec::EnvItem;
use starina::spec::EnvType;
use starina::timer::Timer;

pub const SPEC: AppSpec = AppSpec {
    name: "echo_client",
    env: &[EnvItem {
        name: "echo",
        ty: EnvType::Service { service: "echo" },
    }],
    exports: &[],
    main,
};

#[derive(Debug, Deserialize)]
struct Env {
    pub echo: Channel,
}

enum State {
    EchoChannel(ChannelReceiver),
    Timer(Timer),
}

fn main(environ: Environ) {
    let env: Env = environ.parse().expect("Failed to parse environment");

    let poll = Poll::new().unwrap();
    let (echo_tx, echo_rx) = env.echo.split();
    let timer = Timer::new().unwrap();

    timer.set_timeout(Duration::from_millis(0)).unwrap();

    poll.add(
        echo_rx.handle_id(),
        State::EchoChannel(echo_rx),
        Readiness::READABLE | Readiness::CLOSED,
    )
    .unwrap();

    poll.add(timer.handle_id(), State::Timer(timer), Readiness::READABLE)
        .unwrap();

    info!("started echo client");

    let mut msgbuffer = MessageBuffer::new();
    let mut counter = 0u32;

    loop {
        let (state, readiness) = poll.wait().unwrap();
        match state.as_ref() {
            State::EchoChannel(ch) if readiness.contains(Readiness::READABLE) => {
                match ch.recv(&mut msgbuffer) {
                    Ok(Message::Data { data }) => {
                        info!(
                            "received echo reply: {:?}",
                            core::str::from_utf8(data).unwrap_or("<invalid utf8>")
                        );
                    }
                    Ok(msg) => {
                        debug_warn!("unexpected message: {:?}", msg);
                    }
                    Err(RecvError::Parse(msginfo)) => {
                        debug_warn!("malformed message: {}", msginfo.kind());
                    }
                    Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                    Err(RecvError::Syscall(err)) => {
                        debug_warn!("recv error: {:?}", err);
                        break;
                    }
                }
            }
            State::EchoChannel(_) if readiness == Readiness::CLOSED => {
                info!("connection closed");
                break;
            }
            State::Timer(timer) if readiness.contains(Readiness::READABLE) => {
                let message = format!("value {}", counter);
                echo_tx
                    .send(Message::Data {
                        data: message.as_bytes(),
                    })
                    .unwrap();

                counter += 1;
                timer.set_timeout(Duration::from_millis(2000)).unwrap();
            }
            _ => {
                debug_warn!("unexpected readiness: {:?}", readiness);
            }
        }
    }
}
