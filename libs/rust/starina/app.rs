use hashbrown::HashMap;

use crate::channel::userspace::Channel;
use crate::channel::userspace::message::PingReader;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::poll::Readiness;
use crate::poll::userspace::Poll;

pub trait App: Send + Sync {
    fn init(dispatcher: &Dispatcher, ch: Channel) -> Self
    where
        Self: Sized;

    fn on_ping(&self, ch: &Channel, ping: PingReader);
}

pub enum Object {
    Channel(Channel),
}

pub struct Dispatcher {
    poll: Poll,
    objects: spin::Mutex<HashMap<HandleId, Object>>,
}

impl Dispatcher {
    pub fn new(poll: Poll) -> Self {
        Self {
            poll,
            objects: spin::Mutex::new(HashMap::new()),
        }
    }

    pub fn add_channel(&self, channel: Channel) -> Result<(), ErrorCode> {
        self.poll.add(channel.handle_id(), Readiness::WRITABLE)?;
        self.objects
            .lock()
            .insert(channel.handle_id(), Object::Channel(channel));

        Ok(())
    }

    /// Enables `READABLE` interest. This is the counterpart of `disable_incoming`,
    /// when the app is now ready to process more data from the object.
    pub fn enable_incoming(&self, handle: &impl Handleable) -> Result<(), ErrorCode> {
        todo!()
    }

    /// Disables `READABLE` interest. This is called when the app is not ready to process
    /// more data from the object (i.e. backpressure).
    pub fn disable_incoming(&self, handle: &impl Handleable) -> Result<(), ErrorCode> {
        todo!()
    }

    /// Edge-triggered.
    pub fn watch_writable(&self, handle: &impl Handleable) -> Result<(), ErrorCode> {
        todo!()
    }

    fn wait_and_dispatch(&self, app: &impl App) {
        let (handle, readiness) = self.poll.wait().unwrap();

        let objects = self.objects.lock();
        let object = objects.get(&handle).expect("object not found");

        match object {
            Object::Channel(channel) => {
                if readiness.contains(Readiness::WRITABLE) {
                    // TODO:
                    let msg = channel.recv().unwrap();
                    let ping = PingReader::try_from(msg).unwrap();
                    app.on_ping(channel, ping);
                }
            }
        }
    }
}

pub fn app_loop<A: App>(ch: Channel) {
    let poll = Poll::create().unwrap();
    let dispatcher = Dispatcher::new(poll);
    let app = A::init(&dispatcher, ch);

    let ch = Channel::from_handle(OwnedHandle::from_raw(HandleId::from_raw(1)));
    dispatcher.add_channel(ch).unwrap();

    loop {
        dispatcher.wait_and_dispatch(&app);
    }
}
