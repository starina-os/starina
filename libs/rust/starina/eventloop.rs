use alloc::sync::Arc;
use core::marker::PhantomData;

use hashbrown::HashMap;
use serde::de::DeserializeOwned;
use starina_types::syscall::VsyscallPage;

use crate::channel::Channel;
use crate::channel::ChannelReceiver;
use crate::channel::ChannelSender;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::interrupt::Interrupt;
use crate::message::AnyMessage;
use crate::message::CallId;
use crate::message::ConnectMsg;
use crate::message::FramedDataMsg;
use crate::message::MessageKind;
use crate::message::Messageable;
use crate::message::MessageableWithCallId;
use crate::message::OpenMsg;
use crate::message::OpenReplyMsg;
use crate::message::StreamDataMsg;
use crate::poll::Poll;
use crate::poll::Readiness;
use crate::tls;

/// Trait defining the Dispatcher interface for EventLoop applications
pub trait Dispatcher<St> {
    /// Add a channel to the dispatcher
    fn add_channel(&self, state: St, channel: Channel) -> Result<ChannelSender, ErrorCode>;

    /// Close a channel
    fn close_channel(&self, handle: HandleId) -> Result<(), ErrorCode>;

    /// Add an interrupt to the dispatcher
    fn add_interrupt(&self, state: St, interrupt: Interrupt) -> Result<(), ErrorCode>;
}

pub struct Completer<M>
where
    M: for<'a> MessageableWithCallId<'a>,
{
    call_id: CallId,
    sender: ChannelSender,
    _pd: PhantomData<M>,
    #[cfg(debug_assertions)]
    sent: bool,
}

impl<M> Completer<M>
where
    M: for<'a> MessageableWithCallId<'a>,
{
    fn new(call_id: CallId, sender: ChannelSender) -> Self {
        Self {
            call_id,
            sender,
            _pd: PhantomData,
            #[cfg(debug_assertions)]
            sent: false,
        }
    }

    pub fn reply(mut self, message: M) -> Result<(), ErrorCode> {
        #[cfg(debug_assertions)]
        {
            self.sent = true;
        }

        self.sender.reply(self.call_id, message)
    }
}

impl<M> Drop for Completer<M>
where
    M: for<'a> MessageableWithCallId<'a>,
{
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        if !self.sent {
            panic!(
                "completer is dropped without replying {}",
                core::any::type_name::<M>()
            );
        }
    }
}

pub trait EventLoop: Send + Sync {
    type Env;
    type State: Send + Sync;

    /// Initializes an application.
    ///
    /// This is an equivalent to the `main` function in a normal Rust program,
    /// but in a component-like way.
    fn init(dispatcher: &dyn Dispatcher<Self::State>, env: Self::Env) -> Self
    where
        Self: Sized;

    /// `connect` message handler.
    ///
    /// This message is used when a new channel connection is established to
    /// the application. `msg.handle` is the handle of the new channel.
    ///
    /// This is so-called *passive open*, while `open-reply` message is
    /// *active open*. Close the channel immediately if you don't want
    /// to accept the connection.
    ///
    /// Examples:
    ///
    /// - Kernel sends this message when the app is a server and a new client
    ///   connection is established.
    /// - TCP/IP server sends this message when a new TCP connection is
    ///   established to a listening socket.
    #[allow(unused_variables)]
    fn on_connect(&self, ctx: Context<Self::State>, msg: ConnectMsg) {
        debug_warn!("ignored connect message");
    }

    /// `open` message handler.
    ///
    /// This is an equivalent to the `open(2)` system call in UNIX. Servers
    /// reply with `open-reply` message with the handle of the new channel
    /// to use the opened resource such as files, TCP sockets, etc.
    #[allow(unused_variables)]
    fn on_open(
        &self,
        ctx: Context<Self::State>,
        completer: Completer<OpenReplyMsg>,
        msg: OpenMsg<'_>,
    ) {
        debug_warn!("ignored open message");
    }

    /// `open-reply` message handler.
    ///
    /// This is so-called *active open*, while `connect` message is
    /// *passive open*.
    ///
    /// This message is used to reply to the `open` message. The `msg.handle`
    /// is the handle of the new channel representing the opened resource.
    #[allow(unused_variables)]
    fn on_open_reply(&self, ctx: Context<Self::State>, call_id: CallId, msg: OpenReplyMsg) {
        debug_warn!("ignored open-reply message");
    }

    /// `framed-data` message handler.
    ///
    /// This message is used for datagram-like data transfers such as UDP,
    /// network packets, etc.
    #[allow(unused_variables)]
    fn on_framed_data(&self, _ctx: Context<Self::State>, _msg: FramedDataMsg<'_>) {
        debug_warn!("ignored framed data message");
    }

    /// `stream-data` message handler.
    ///
    /// This message is used for byte-stream-like data transfers such as TCP,
    /// stdio/stderr, UNIX pipes, etc.
    #[allow(unused_variables)]
    fn on_stream_data(&self, _ctx: Context<Self::State>, _msg: StreamDataMsg<'_>) {
        debug_warn!("ignored stream data message");
    }

    // Callback for unknown messages.
    #[allow(unused_variables)]
    fn on_unknown_message(&self, ctx: Context<Self::State>, msg: AnyMessage) {
        debug_warn!("ignored message: {}", msg.msginfo.kind());
    }

    /// Interrupt handler.
    #[allow(unused_variables)]
    fn on_interrupt(&self, interrupt: &Interrupt) {
        debug_warn!("ignored interrupt");
    }
}

pub enum Object {
    Channel {
        receiver: ChannelReceiver,
        sender: ChannelSender,
    },
    Interrupt {
        interrupt: Interrupt,
    },
}

pub struct Context<'a, St> {
    pub sender: &'a ChannelSender,
    pub dispatcher: &'a dyn Dispatcher<St>,
    pub state: &'a mut St,
}

pub struct ObjectWithState<St> {
    pub object: Object,
    pub state: spin::Mutex<St>,
}

/// A dispatcher that uses `Poll` to wait for events.
pub struct PollDispatcher<St> {
    poll: Poll,
    objects: spin::RwLock<HashMap<HandleId, Arc<ObjectWithState<St>>>>,
}

impl<St> PollDispatcher<St> {
    pub fn new(poll: Poll) -> Self {
        Self {
            poll,
            objects: spin::RwLock::new(HashMap::new()),
        }
    }

    fn wait_and_dispatch<E>(&self, app: &impl EventLoop<Env = E, State = St>) {
        let (handle, readiness) = self.poll.wait().unwrap();

        let object = {
            let objects_lock = self.objects.read();
            objects_lock
                .get(&handle)
                .cloned()
                .expect("object not found")
        };

        match &object.object {
            Object::Channel { receiver, sender } => {
                if readiness.contains(Readiness::READABLE) {
                    let mut msg = receiver.recv().unwrap();
                    let mut state_lock = object.state.lock();
                    let ctx = Context {
                        sender,
                        dispatcher: self,
                        state: &mut *state_lock,
                    };

                    match msg.msginfo.kind() {
                        kind if kind == MessageKind::Connect as usize => {
                            match unsafe {
                                ConnectMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_connect(ctx, msg),
                                None => {
                                    app.on_unknown_message(ctx, msg);
                                }
                            };
                        }
                        kind if kind == MessageKind::Open as usize => {
                            match unsafe { OpenMsg::parse_unchecked(msg.msginfo, &mut msg.buffer) }
                            {
                                Some(msg) => {
                                    let completer = Completer::new(todo!(), ctx.sender.clone());
                                    app.on_open(ctx, completer, msg)
                                }
                                None => {
                                    app.on_unknown_message(ctx, msg);
                                }
                            };
                        }
                        kind if kind == MessageKind::OpenReply as usize => {
                            match unsafe {
                                OpenReplyMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_open_reply(ctx, todo!(), msg),
                                None => {
                                    app.on_unknown_message(ctx, msg);
                                }
                            };
                        }
                        kind if kind == MessageKind::FramedData as usize => {
                            match unsafe {
                                FramedDataMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_framed_data(ctx, msg),
                                None => {
                                    app.on_unknown_message(ctx, msg);
                                }
                            };
                        }
                        kind if kind == MessageKind::StreamData as usize => {
                            match unsafe {
                                StreamDataMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_stream_data(ctx, msg),
                                None => {
                                    app.on_unknown_message(ctx, msg);
                                }
                            };
                        }
                        _ => panic!("unexpected message kind: {}", msg.msginfo.kind()),
                    }
                }
            }
            Object::Interrupt { interrupt } => {
                if readiness.contains(Readiness::READABLE) {
                    app.on_interrupt(interrupt);
                }
            }
        }
    }
}

impl<St> Dispatcher<St> for PollDispatcher<St> {
    fn add_channel(&self, state: St, channel: Channel) -> Result<ChannelSender, ErrorCode> {
        let handle_id = channel.handle_id();

        // Tell the kernel to notify us when the channel is readable.
        self.poll.add(handle_id, Readiness::READABLE)?;

        // Register the channel in the dispatcher.
        let (sender, receiver) = channel.split();
        let object = Object::Channel {
            sender: sender.clone(),
            receiver,
        };
        self.objects.write().insert(
            handle_id,
            Arc::new(ObjectWithState {
                object,
                state: spin::Mutex::new(state),
            }),
        );

        Ok(sender)
    }

    fn close_channel(&self, handle: HandleId) -> Result<(), ErrorCode> {
        // Remove the channel from the dispatcher.
        self.objects.write().remove(&handle);

        // Tell the kernel to stop notifying us about this channel.
        self.poll.remove(handle)?;

        Ok(())
    }

    fn add_interrupt(&self, state: St, interrupt: Interrupt) -> Result<(), ErrorCode> {
        let handle_id = interrupt.handle_id();
        self.poll.add(handle_id, Readiness::READABLE)?;
        let object = Object::Interrupt { interrupt };
        self.objects.write().insert(
            handle_id,
            Arc::new(ObjectWithState {
                object,
                state: spin::Mutex::new(state),
            }),
        );

        Ok(())
    }
}

pub fn app_loop<Env, St, A>(program_name: &'static str, vsyscall: *const VsyscallPage) -> !
where
    Env: DeserializeOwned,
    St: Send + Sync,
    A: EventLoop<Env = Env, State = St>,
{
    tls::init_thread_local(program_name);

    let env_json = unsafe {
        let ptr = (*vsyscall).environ_ptr;
        let len = (*vsyscall).environ_len;
        core::slice::from_raw_parts(ptr, len)
    };

    let env: Env = serde_json::from_slice(env_json).expect("failed to parse env");

    let poll = Poll::create().unwrap();
    let dispatcher = PollDispatcher::new(poll);
    let app: A = A::init(&dispatcher, env);

    loop {
        dispatcher.wait_and_dispatch(&app);
    }
}
