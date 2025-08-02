use alloc::boxed::Box;

use serde::Deserialize;
use starina_types::environ::Environ;
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::poll::Readiness;

use crate::channel::Channel;
use crate::channel::RecvError;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::message::Message;
use crate::message::MessageBuffer;
use crate::poll::Poll;

pub enum Item {
    Channel {
        ch: Channel,
        this: Box<dyn ChannelHandler>,
    },
    Startup {
        ch: Channel,
    },
}

pub struct Dispatcher<'a>(&'a Poll<Item>);

impl<'a> Dispatcher<'a> {
    pub fn add_channel(
        &self,
        ch: Channel,
        handler: impl ChannelHandler + 'static,
    ) -> Result<(), Error> {
        self.0
            .add(
                ch.handle_id(),
                Item::Channel {
                    ch,
                    this: Box::new(handler),
                },
                Readiness::READABLE | Readiness::CLOSED,
            )
            .map_err(Error::PollAdd)?;
        Ok(())
    }
}

pub struct ChannelContext<'a> {
    pub dispatcher: &'a Dispatcher<'a>,
    pub ch: &'a Channel,
}

pub struct StartupContext<'a> {
    pub dispatcher: &'a Dispatcher<'a>,
}

pub trait StartupHandler<E>
where
    E: for<'a> Deserialize<'a>,
{
    fn init(ctx: &StartupContext, env: E) -> Self;
    fn connected(&self, ctx: &StartupContext, ch: Channel);
}

pub trait ChannelHandler {
    /// Returns `true` if we can accept a message on this channel. If not,
    /// the dispatcher does not receive a message (backpressure) until you
    /// resume receiving.
    fn is_receivable(&self, _ctx: &ChannelContext<'_>) -> bool {
        true
    }

    fn data(&self, _ctx: &ChannelContext<'_>, _data: &[u8]) {}
    fn disconnected(&self, _ctx: &ChannelContext<'_>) {}
}

pub struct EventLoop {
    poll: Poll<Item>,
}

#[derive(Debug)]
pub enum Error {
    PollCreate(ErrorCode),
    PollAdd(ErrorCode),
    PollWait(ErrorCode),
}

impl EventLoop {
    pub fn new(startup_ch: Channel) -> Result<Self, Error> {
        let poll = Poll::new().map_err(Error::PollCreate)?;
        poll.add(
            startup_ch.handle_id(),
            Item::Startup { ch: startup_ch },
            Readiness::READABLE | Readiness::CLOSED,
        )
        .map_err(Error::PollAdd)?;
        Ok(Self { poll })
    }

    pub fn run<A, E>(&mut self, app: &A) -> Result<(), Error>
    where
        A: StartupHandler<E>,
        E: for<'a> Deserialize<'a>,
    {
        let mut msgbuffer = MessageBuffer::new();
        loop {
            let (state, readiness) = self.poll.wait().map_err(Error::PollWait)?;
            match &*state {
                Item::Startup { ch } => {
                    let dispatcher = Dispatcher(&self.poll);
                    let ctx = StartupContext {
                        dispatcher: &dispatcher,
                    };

                    if readiness.contains(Readiness::READABLE) {
                        match ch.recv(&mut msgbuffer) {
                            Ok(Message::Connect { ch }) => {
                                app.connected(&ctx, ch);
                            }
                            Ok(_) => {
                                todo!()
                            }
                            Err(RecvError::Parse(msginfo)) => {
                                debug_warn!("malformed message from startup: {}", msginfo.kind());
                            }
                            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                            Err(RecvError::Syscall(err)) => {
                                debug_warn!("recv error from startup: {:?}", err);
                            }
                        }
                    }
                }
                Item::Channel { ch, this } => {
                    let dispatcher = Dispatcher(&self.poll);
                    let ctx = ChannelContext {
                        dispatcher: &dispatcher,
                        ch,
                    };

                    // Receive a message.
                    if readiness.contains(Readiness::READABLE) && this.is_receivable(&ctx) {
                        match ch.recv(&mut msgbuffer) {
                            Ok(Message::Data { data }) => {
                                this.data(&ctx, data);
                            }
                            Ok(_) => {
                                todo!()
                            }
                            Err(RecvError::Parse(msginfo)) => {
                                debug_warn!("malformed message on client: {}", msginfo.kind());
                            }
                            Err(RecvError::Syscall(ErrorCode::Empty)) => {}
                            Err(RecvError::Syscall(err)) => {
                                debug_warn!("recv error on client: {:?}", err);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run<A, E>(environ: Environ)
where
    A: StartupHandler<E>,
    E: for<'a> Deserialize<'a>,
{
    let env_json: serde_json::Value = serde_json::from_slice(environ.raw()).unwrap();
    let startup_ch = env_json
        .get("startup_ch")
        .expect("startup_ch not found")
        .as_i64()
        .and_then(|i| i.try_into().ok())
        .map(HandleId::from_raw)
        .map(OwnedHandle::from_raw)
        .map(Channel::from_handle)
        .unwrap();

    let mut eventloop = EventLoop::new(startup_ch).unwrap();
    let dispatcher = Dispatcher(&eventloop.poll);
    let ctx = StartupContext {
        dispatcher: &dispatcher,
    };

    let env: E = serde_json::from_value(env_json).expect("Failed to parse environment");
    let app = A::init(&ctx, env);

    eventloop.run(&app).unwrap();
}
