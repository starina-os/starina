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
    ChannelAlreadyAdded(Channel),
    ChannelReceiverAlreadyAdded((ChannelReceiver, ChannelSender)),
    InterruptAlreadyAdded(Interrupt),
}

#[derive(Debug)]
pub enum Event<'a, Ctx, M: MessageDeserialize> {
    Message(&'a mut Ctx, M::Reader<'a>, &'a mut ChannelSender),
    Interrupt(&'a mut Ctx, &'a mut Interrupt),
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

    pub fn add_channel(&mut self, ch: Channel, ctx: Ctx) -> Result<(), Error> {
        let handle_id = ch.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::ChannelAlreadyAdded(ch));
        }

        let (sender, receiver) = ch.split();
        let entry = Entry {
            ctx,
            object: Object::Channel { sender, receiver },
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    pub fn remove(&mut self, handle_id: HandleId) {
        self.objects.remove(&handle_id);
        // TODO:
        // self.poll.remove()
    }

    pub fn add_channel_receiver(
        &mut self,
        receiver: ChannelReceiver,
        sender: ChannelSender,
        state: Ctx,
    ) -> Result<(), Error> {
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
                    let m = match receiver.recv_with_buffer::<AllM>(&mut self.msgbuffer) {
                        Ok(m) => m,
                        Err(err) => return Event::Error(Error::ChannelRecv(err)),
                    };

                    return Event::Message(&mut entry.ctx, m, sender);
                }
                Object::Interrupt(interrupt) => {
                    return Event::Interrupt(&mut entry.ctx, interrupt);
                }
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
