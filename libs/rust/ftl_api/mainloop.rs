use core::marker::PhantomData;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageDeserialize;
use ftl_types::poll::PollEvent;
use hashbrown::HashMap;

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

#[derive(Debug)]
pub enum Event<'a, Ctx, M: MessageDeserialize> {
    Message {
        ctx: &'a mut Ctx,
        message: M::Reader<'a>,
        sender: &'a mut ChannelSender,
    },
    Interrupt {
        ctx: &'a mut Ctx,
        interrupt: &'a mut Interrupt,
    },
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
    ctx: Ctx,
    object: Object,
}

pub struct Mainloop<Ctx, AllM> {
    poll: Poll,
    objects: HashMap<HandleId, Entry<Ctx>>,
    msgbuffer: MessageBuffer,
    _pd: PhantomData<AllM>,
}

impl<Ctx, AllM: MessageDeserialize> Mainloop<Ctx, AllM> {
    pub fn new() -> Result<Self, Error> {
        let poll = Poll::new().map_err(Error::PollCreate)?;

        Ok(Self {
            poll,
            objects: HashMap::new(),
            msgbuffer: MessageBuffer::new(),
            _pd: PhantomData,
        })
    }

    pub fn remove(&mut self, handle_id: HandleId) -> Result<(), FtlError> {
        self.poll.remove(handle_id)?;
        self.objects.remove(&handle_id); // TODO: warn if not found
        Ok(())
    }

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
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    pub fn add_interrupt(&mut self, interrupt: Interrupt, state: Ctx) -> Result<(), Error> {
        let handle_id = interrupt.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::InterruptAlreadyAdded(interrupt));
        }

        let entry = Entry {
            ctx: state,
            object: Object::Interrupt(interrupt),
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    pub fn next(&mut self) -> Event<'_, Ctx, AllM> {
        let (poll_ev, handle_id) = match self.poll.wait() {
            Ok(ev) => ev,
            Err(err) => return Event::Error(Error::PollWait(err)),
        };

        let entry = self.objects.get_mut(&handle_id).unwrap();
        if poll_ev.contains(PollEvent::READABLE) {
            match &mut entry.object {
                Object::Channel { sender, receiver } => {
                    let message = match receiver.try_recv_with_buffer::<AllM>(&mut self.msgbuffer) {
                        Ok(Some(m)) => m,
                        Ok(None) => return Event::Error(Error::ChannelRecvWouldBlock),
                        Err(err) => return Event::Error(Error::ChannelRecv(err)),
                    };

                    return Event::Message {
                        ctx: &mut entry.ctx,
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
