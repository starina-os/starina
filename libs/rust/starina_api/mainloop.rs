//! The mainloop for applications.
use core::marker::PhantomData;

use hashbrown::HashMap;
use starina_types::error::FtlError;
use starina_types::handle::HandleId;
use starina_types::message::MessageBuffer;
use starina_types::message::MessageDeserialize;
use starina_types::poll::PollEvent;

use crate::channel::Channel;
use crate::channel::ChannelReceiver;
use crate::channel::ChannelSender;
use crate::channel::RecvError;
use crate::interrupt::Interrupt;
use crate::poll::Poll;

#[derive(Debug)]
pub enum Error {
    PollCreate(FtlError),
    PollAdd(FtlError),
    PollWait(FtlError),
    ChannelRecv(RecvError),
    ChannelRecvWouldBlock,
    ChannelAlreadyAdded(Channel),
    ChannelReceiverAlreadyAdded((ChannelReceiver, ChannelSender)),
    InterruptAlreadyAdded(Interrupt),
}

/// Events that applications need to handle.
#[derive(Debug)]
#[non_exhaustive]
pub enum Event<'a, Ctx, M: MessageDeserialize> {
    /// A received message.
    Message {
        /// The per-object state associated with the channel object.
        ctx: &'a mut Ctx,
        /// The received message.
        message: M::Reader<'a>,
        /// The channel where the message is received.
        sender: &'a mut ChannelSender,
        /// The handle ID of the channel.
        handle_id: HandleId,
    },
    /// A received hardware interrupts.
    Interrupt {
        /// The object which received the interrupt.
        interrupt: &'a mut Interrupt,
        /// The per-object state associated with the interrupt object.
        ctx: &'a mut Ctx,
    },
    /// An error occurred when processing events.
    Error(Error),
}

enum Object {
    Channel {
        receiver: ChannelReceiver,
        sender: ChannelSender,
    },
    Interrupt(Interrupt),
}

struct Entry<Ctx> {
    handle_id: HandleId,
    ctx: Ctx,
    object: Object,
}

/// The mainloop for applications.
///
/// This is a simple event loop to enable asynchronous programming without
/// Rust's `async fn`s. It is designed to be used in the `main` function of
/// applications.
///
/// # Per-object state
///
/// Each object in the mainloop has its own state. For example, a TCP socket
/// channel would have a state to track the connection state, timers, and
/// TX/RX buffers.
///
/// See the example below for how to define and use per-object state.
///
/// # Why not async Rust?
///
/// This API is very similar to `epoll` + non-blocking I/O in Linux. An event
/// loop API like this means that you need to write state machines manually, which
/// async Rust (`async fn`) does automatically.
///
/// However, explicit state machines make debugging easier because the
/// execution flow is crystal clear. Also we don't have to care about pitfalls like
/// *cancellation safety*. Moreover, my observation is that most of
/// OS components are very simple and manual state machines are sufficient.
///
/// # Future work
///
/// - Support multi-threaded mainloop.
///
/// # Example
///
/// ```
/// starina_api::autogen!(); // Include starina_autogen module
///
/// use starina_api::channel::Channel;
/// use starina_api::environ::Environ;
/// use starina_api::mainloop::Event;
/// use starina_api::mainloop::Mainloop;
/// use starina_api::prelude::*;
/// use starina_autogen::idl::ping::PingReply;
/// use starina_autogen::idl::Message;
///
/// // Per-object state.
/// #[derive(Debug)]
/// enum Context {
///     Startup,
///     Client { counter: i32 },
/// }
///
/// #[no_mangle]
/// pub fn main(mut env: Environ) {
///     let mut mainloop = Mainloop::<Context, Message>::new().unwrap();
///
///     // Take the startup channel, and start receiving messages through the
///     // mainloop.
///     let startup_ch = env.take_channel("dep:startup").unwrap();
///     mainloop.add_channel(startup_ch, Context::Startup).unwrap();
///
///     // Mainloop!
///     loop {
///        // Wait for the next event. Use `match` not to miss unexpected cases.
///         match mainloop.next() {
///             Event::Message { // The "message received" event.
///                 ctx: Context::Startup, // The message is from startup.
///                 message: Message::NewClient(m), // NewClient message.
///                 ..
///             } => {
///                 // Take the new client's channel and register it to the
///                 // mainloop.
///                 let new_ch = m.handle.take::<Channel>().unwrap();
///                 mainloop
///                     .add_channel(new_ch, Context::Client { counter: 0 })
///                     .unwrap();
///             }
///             Event::Message { // The "message received" event.
///                 ctx: Context::Client { counter }, // The message is from a client.
///                 message: Message::Ping(m), // Ping message.
///                 sender, // The channel which received the message.
///             } => {
///                 // Update the per-object state. It's mutable!
///                 *counter += 1;
///
///                 // Reply with the counter value.
///                 if let Err(err) = sender.send(PingReply { value: *counter }) {
///                     debug_warn!("failed to reply: {:?}", err);
///                 }
///             }
///             ev => {
///                 panic!("unexpected event: {:?}", ev);
///             }
///         }
///     }
/// }
/// ```
pub struct Mainloop<Ctx, AllM> {
    poll: Poll,
    objects: HashMap<HandleId, Entry<Ctx>>,
    msgbuffer: MessageBuffer,
    _pd: PhantomData<AllM>,
}

impl<Ctx, AllM: MessageDeserialize> Mainloop<Ctx, AllM> {
    /// Creates a new mainloop.
    pub fn new() -> Result<Self, Error> {
        let poll = Poll::new().map_err(Error::PollCreate)?;

        Ok(Self {
            poll,
            objects: HashMap::new(),
            msgbuffer: MessageBuffer::new(),
            _pd: PhantomData,
        })
    }

    /// Removes an object.
    pub fn remove(&mut self, handle_id: HandleId) -> Result<(), FtlError> {
        self.poll.remove(handle_id)?;
        self.objects.remove(&handle_id); // TODO: warn if not found
        Ok(())
    }

    /// Adds a channel to start receiving messages in the mainloop.
    pub fn add_channel<T: Into<(ChannelSender, ChannelReceiver)>>(
        &mut self,
        channel: T,
        state: Ctx,
    ) -> Result<(), Error> {
        let (sender, receiver) = channel.into();
        let handle_id = receiver.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::ChannelReceiverAlreadyAdded((receiver, sender)));
        }

        let entry = Entry {
            ctx: state,
            object: Object::Channel { receiver, sender },
            handle_id,
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    /// Adds an interrupt to start receiving interrupts in the mainloop.
    pub fn add_interrupt(&mut self, interrupt: Interrupt, state: Ctx) -> Result<(), Error> {
        let handle_id = interrupt.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::InterruptAlreadyAdded(interrupt));
        }

        let entry = Entry {
            ctx: state,
            object: Object::Interrupt(interrupt),
            handle_id,
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    /// Waits for the next event. Blocks until an event is available.
    pub fn next(&mut self) -> Event<'_, Ctx, AllM> {
        let (poll_ev, handle_id) = match self.poll.wait() {
            Ok(ev) => ev,
            Err(err) => return Event::Error(Error::PollWait(err)),
        };

        let entry = self.objects.get_mut(&handle_id).unwrap();
        if poll_ev.contains(PollEvent::READABLE) {
            match &mut entry.object {
                Object::Channel { sender, receiver } => {
                    let message = match receiver.try_recv::<AllM>(&mut self.msgbuffer) {
                        Ok(Some(m)) => m,
                        Ok(None) => return Event::Error(Error::ChannelRecvWouldBlock),
                        Err(err) => return Event::Error(Error::ChannelRecv(err)),
                    };

                    return Event::Message {
                        ctx: &mut entry.ctx,
                        handle_id: entry.handle_id,
                        message,
                        sender,
                    };
                }
                Object::Interrupt(interrupt) => {
                    return Event::Interrupt {
                        ctx: &mut entry.ctx,
                        interrupt,
                    };
                }
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
