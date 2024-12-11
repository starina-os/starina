use hashbrown::HashMap;
use starina_types::{error::ErrorCode, handle::HandleId, message::MessageBuffer, poll::PollEvent};

use crate::{channel::{ChannelReceiver, ChannelSender}, poll::Poll};

#[derive(Debug)]
pub enum Message<'a> {
    /// A message that contains a string.
    String(&'a str),
    /// A message that contains a byte array.
    Bytes(&'a [u8]),
}

#[derive(Debug)]
pub enum Error {
    /// An error while waiting for or reading an event.
    PollWait(ErrorCode),
    /// An error while receiving a message from a channel.
    ChannelRecv(ErrorCode),
    /// The channel receive operation would block.
    ChannelRecvWouldBlock,
}

/// Events that applications need to handle.
#[derive(Debug)]
pub enum Event<'a, Ctx> {
    /// An error while waiting for or reading an event.
    Error(Error),
    /// A received message.
    Message {
        /// The per-object state associated with the channel object.
        ctx: &'a mut Ctx,
        /// The received message.
        message: Message<'a>,
        /// The channel where the message is received.
        sender: &'a mut ChannelSender,
        /// The handle ID of the channel.
        handle_id: HandleId,
    },
}

enum Object {
    Channel {
        receiver: ChannelReceiver,
        sender: ChannelSender,
    },
}

struct Entry<Ctx> {
    handle_id: HandleId,
    ctx: Ctx,
    object: Object,
}

pub struct Mainloop<Ctx> {
    poll: Poll,
    msgbuffer: MessageBuffer,
    objects: HashMap<HandleId, Entry<Ctx>>,
}

impl<Ctx> Mainloop<Ctx> {
    pub fn new() -> Result<Mainloop<Ctx>, Error> {
        todo!()
    }

    /// Waits for the next event. Blocks until an event is available.
    pub fn next(&mut self) -> Event<'_, Ctx> {
        let (poll_ev, handle_id) = match self.poll.wait() {
            Ok(ev) => ev,
            Err(err) => return Event::Error(Error::PollWait(err)),
        };

        let entry = self.objects.get_mut(&handle_id).unwrap();
        if poll_ev.contains(PollEvent::READABLE) {
            match &mut entry.object {
                Object::Channel { sender, receiver } => {
                    let message = match receiver.try_recv(&mut self.msgbuffer) {
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
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
