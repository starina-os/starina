use core::marker::PhantomData;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageBuffer;
use ftl_types::message::MessageDeserialize;
use ftl_types::poll::PollEvent;
use hashbrown::HashMap;

use crate::channel::Channel;
use crate::channel::RecvError;
use crate::poll::Poll;

#[derive(Debug)]
pub enum Error {
    PollCreate(FtlError),
    PollAdd(FtlError),
    PollWait(FtlError),
    ChannelRecv(RecvError),
    ChannelAlreadyAdded(Channel),
}

#[derive(Debug)]
pub enum Event<'a, 'b, St, M: MessageDeserialize> {
    Message {
        ctx: &'a mut St,
        ch: &'a mut Channel,
        m: M::Reader<'b>,
    },
    Error(Error),
}

enum Object {
    Channel(Channel),
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

        let entry = Entry {
            ctx: state,
            object: Object::Channel(ch),
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
                Object::Channel(ch) => {
                    let m = match ch.recv_with_buffer::<AllM>(msgbuffer) {
                        Ok(m) => m,
                        Err(err) => return Event::Error(Error::ChannelRecv(err)),
                    };

                    return Event::Message {
                        ch,
                        ctx: &mut entry.ctx,
                        m,
                    };
                }
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
