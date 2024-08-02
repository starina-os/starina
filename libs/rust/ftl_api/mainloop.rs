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
pub enum Event<'a, 'b, St, M: MessageDeserialize> {
    Message {
        ctx: &'a mut St,
        sender: &'a mut ChannelSender,
        m: M::Reader<'b>,
    },
    Interrupt {
        ctx: &'a mut St,
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

struct Entry<St> {
    ctx: St,
    object: Object,
}

pub struct Mainloop<St, AllM> {
    poll: Poll,
    objects: HashMap<HandleId, Entry<St>>,
    _pd: PhantomData<AllM>,
}

impl<Ctx, AllM: MessageDeserialize> Mainloop<Ctx, AllM> {
    pub fn new() -> Result<Self, Error> {
        let poll = Poll::new().map_err(Error::PollCreate)?;

        Ok(Self {
            poll,
            objects: HashMap::new(),
            _pd: PhantomData,
        })
    }

    pub fn add_channel(&mut self, ch: Channel, state: Ctx) -> Result<(), Error> {
        let handle_id = ch.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::ChannelAlreadyAdded(ch));
        }

        let (sender, receiver) = ch.split();
        let entry = Entry {
            ctx: state,
            object: Object::Channel { sender, receiver },
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
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

    pub fn next<'a, 'b>(
        &'a mut self,
        msgbuffer: &'b mut MessageBuffer,
    ) -> Event<'a, 'b, Ctx, AllM> {
        let (poll_ev, handle_id) = match self.poll.wait() {
            Ok(ev) => ev,
            Err(err) => return Event::Error(Error::PollWait(err)),
        };

        let entry = self.objects.get_mut(&handle_id).unwrap();
        if poll_ev.contains(PollEvent::READABLE) {
            match &mut entry.object {
                Object::Channel { sender, receiver } => {
                    let m = match receiver.recv_with_buffer::<AllM>(msgbuffer) {
                        Ok(m) => m,
                        Err(err) => return Event::Error(Error::ChannelRecv(err)),
                    };

                    return Event::Message {
                        sender,
                        ctx: &mut entry.ctx,
                        m,
                    };
                }
                Object::Interrupt(interrupt) => {
                    return Event::Interrupt {
                        interrupt,
                        ctx: &mut entry.ctx,
                    };
                }
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
