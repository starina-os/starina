use core::marker::PhantomData;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::message::MessageBody;
use ftl_types::message::MessageBuffer;
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
pub enum Event<'a, St, M: MessageBody> {
    Message {
        ch: &'a mut Channel,
        state: &'a mut St,
        m: M::Reader<'a>,
    },
}

enum Object {
    Channel(Channel),
}

struct Entry<St> {
    state: St,
    object: Object,
}

pub struct Mainloop<St, AllM> {
    poll: Poll,
    msgbuffer: MessageBuffer,
    objects: HashMap<HandleId, Entry<St>>,
    _pd: PhantomData<AllM>,
}

impl<St, AllM: MessageBody> Mainloop<St, AllM> {
    pub fn new() -> Result<Self, Error> {
        let poll = Poll::new().map_err(Error::PollCreate)?;

        Ok(Self {
            poll,
            msgbuffer: MessageBuffer::new(),
            objects: HashMap::new(),
            _pd: PhantomData,
        })
    }

    pub fn add_channel(&mut self, ch: Channel, state: St) -> Result<(), Error> {
        let handle_id = ch.handle().id();
        if self.objects.contains_key(&handle_id) {
            return Err(Error::ChannelAlreadyAdded(ch));
        }

        let entry = Entry {
            state,
            object: Object::Channel(ch),
        };

        self.objects.insert(handle_id, entry);
        self.poll
            .add(handle_id, PollEvent::READABLE)
            .map_err(Error::PollAdd)?;

        Ok(())
    }

    pub fn next(&mut self) -> Result<Event<'_, St, AllM>, Error> {
        let (poll_ev, handle_id) = self.poll.wait().map_err(Error::PollWait)?;
        let entry = self.objects.get_mut(&handle_id).unwrap();
        if poll_ev.contains(PollEvent::READABLE) {
            match &mut entry.object {
                Object::Channel(ch) => {
                    let message = ch
                        .recv_with_buffer::<AllM>(&mut self.msgbuffer)
                        .map_err(Error::ChannelRecv)?;
                    return Ok(Event::Message {
                        ch,
                        state: &mut entry.state,
                        m: message,
                    });
                }
            }
        }

        todo!("unhandled poll event: {:?}", poll_ev);
    }
}
