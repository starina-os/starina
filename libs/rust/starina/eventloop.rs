use alloc::sync::Arc;

use hashbrown::HashMap;
use serde::de::DeserializeOwned;
use starina_types::message::MessageKind;
use starina_types::message::Open;
use starina_types::syscall::VsyscallPage;

use crate::channel::Channel;
use crate::channel::ChannelReceiver;
use crate::channel::ChannelSender;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::message::AnyMessage;
use crate::message::Message;
use crate::poll::Poll;
use crate::poll::Readiness;

pub trait EventLoop<E>: Send + Sync {
    fn init(dispatcher: &Dispatcher, env: E) -> Self
    where
        Self: Sized;

    fn on_open(&self, ctx: &Context, msg: Message<Open<'_>>);

    #[allow(unused_variables)]
    fn on_unknown_message(&self, ctx: &Context, msg: AnyMessage) {
        debug_warn!("ignored message: {}", msg.msginfo.kind());
    }
}

pub enum Object {
    Channel {
        receiver: ChannelReceiver,
        sender: ChannelSender,
    },
}

pub struct Context<'a> {
    pub sender: &'a ChannelSender,
    pub dispatcher: &'a Dispatcher,
}

pub struct Dispatcher {
    poll: Poll,
    objects: spin::RwLock<HashMap<HandleId, Arc<spin::Mutex<Object>>>>,
}

impl Dispatcher {
    pub fn new(poll: Poll) -> Self {
        Self {
            poll,
            objects: spin::RwLock::new(HashMap::new()),
        }
    }

    pub fn add_channel(&self, channel: Channel) -> Result<(), ErrorCode> {
        // Tell the kernel to notify us when the channel is readable.
        self.poll.add(channel.handle_id(), Readiness::READABLE)?;

        // Register the channel in the dispatcher.
        let handle_id = channel.handle_id();
        let (sender, receiver) = channel.split();
        let object = Object::Channel { sender, receiver };
        self.objects
            .write()
            .insert(handle_id, Arc::new(spin::Mutex::new(object)));

        Ok(())
    }

    fn wait_and_dispatch<E>(&self, app: &impl EventLoop<E>) {
        let (handle, readiness) = self.poll.wait().unwrap();

        // TODO: Let poll API return an opaque pointer to the object so that
        //       we don't need to have this read-write lock.
        let object_lock = {
            let objects = self.objects.read();
            objects.get(&handle).cloned().expect("object not found")
        };

        let object = object_lock.lock();
        match &*object {
            Object::Channel { receiver, sender } => {
                if readiness.contains(Readiness::READABLE) {
                    let msg = receiver.recv().unwrap();
                    let ctx = Context {
                        sender,
                        dispatcher: self,
                    };

                    match msg.msginfo.kind() {
                        kind @ _ if kind == MessageKind::Open as usize => {
                            match msg.try_into() {
                                Ok(msg) => app.on_open(&ctx, msg),
                                Err(msg) => {
                                    app.on_unknown_message(&ctx, msg);
                                }
                            };
                        }
                        _ => panic!("unexpected message kind: {}", msg.msginfo.kind()),
                    }
                }
            }
        }
    }
}

pub fn app_loop<E: DeserializeOwned, A: EventLoop<E>>(vsyscall: *const VsyscallPage) {
    let env_json = unsafe {
        let ptr = (*vsyscall).environ_ptr;
        let len = (*vsyscall).environ_len;
        core::slice::from_raw_parts(ptr, len)
    };

    let env: E = serde_json::from_slice(&env_json).expect("failed to parse env");

    let poll = Poll::create().unwrap();
    let dispatcher = Dispatcher::new(poll);
    let app = A::init(&dispatcher, env);

    loop {
        dispatcher.wait_and_dispatch(&app);
    }
}
