use alloc::sync::Arc;

use hashbrown::HashMap;
use serde::de::DeserializeOwned;
use starina_types::syscall::VsyscallPage;

use crate::channel::Channel;
use crate::channel::ChannelReceiver;
use crate::channel::ChannelSender;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::interrupt::Interrupt;
use crate::message::AnyMessage;
use crate::message::ConnectMsg;
use crate::message::FramedDataMsg;
use crate::message::MessageKind;
use crate::message::Messageable;
use crate::message::OpenMsg;
use crate::poll::Poll;
use crate::poll::Readiness;
use crate::tls;

pub trait EventLoop<E>: Send + Sync {
    fn init(dispatcher: &Dispatcher, env: E) -> Self
    where
        Self: Sized;

    #[allow(unused_variables)]
    fn on_connect(&self, ctx: &Context, msg: ConnectMsg) {
        debug_warn!("ignored connect message");
    }

    #[allow(unused_variables)]
    fn on_open(&self, ctx: &Context, msg: OpenMsg<'_>) {
        debug_warn!("ignored open message");
    }

    #[allow(unused_variables)]
    fn on_framed_data(&self, _ctx: &Context, _msg: FramedDataMsg<'_>) {
        debug_warn!("ignored framed data message");
    }

    #[allow(unused_variables)]
    fn on_unknown_message(&self, ctx: &Context, msg: AnyMessage) {
        debug_warn!("ignored message: {}", msg.msginfo.kind());
    }

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

    pub fn split_and_add_channel(&self, channel: Channel) -> Result<ChannelSender, ErrorCode> {
        let handle_id = channel.handle_id();

        // Tell the kernel to notify us when the channel is readable.
        self.poll.add(handle_id, Readiness::READABLE)?;

        // Register the channel in the dispatcher.
        let (sender, receiver) = channel.split();
        let object = Object::Channel {
            sender: sender.clone(),
            receiver,
        };
        self.objects
            .write()
            .insert(handle_id, Arc::new(spin::Mutex::new(object)));

        Ok(sender)
    }

    pub fn add_channel(&self, channel: Channel) -> Result<(), ErrorCode> {
        let handle_id = channel.handle_id();

        // Tell the kernel to notify us when the channel is readable.
        self.poll.add(handle_id, Readiness::READABLE)?;

        // Register the channel in the dispatcher.
        let (sender, receiver) = channel.split();
        let object = Object::Channel { sender, receiver };
        self.objects
            .write()
            .insert(handle_id, Arc::new(spin::Mutex::new(object)));

        Ok(())
    }

    pub fn add_interrupt(&self, interrupt: Interrupt) -> Result<(), ErrorCode> {
        let handle_id = interrupt.handle_id();
        self.poll.add(handle_id, Readiness::READABLE)?;
        let object = Object::Interrupt { interrupt };
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
                    let mut msg = receiver.recv().unwrap();
                    let ctx = Context {
                        sender,
                        dispatcher: self,
                    };

                    match msg.msginfo.kind() {
                        kind @ _ if kind == MessageKind::Connect as usize => {
                            match unsafe {
                                ConnectMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_connect(&ctx, msg),
                                None => {
                                    app.on_unknown_message(&ctx, msg);
                                }
                            };
                        }
                        kind @ _ if kind == MessageKind::Open as usize => {
                            match unsafe { OpenMsg::parse_unchecked(msg.msginfo, &mut msg.buffer) }
                            {
                                Some(msg) => app.on_open(&ctx, msg),
                                None => {
                                    app.on_unknown_message(&ctx, msg);
                                }
                            };
                        }
                        kind @ _ if kind == MessageKind::FramedData as usize => {
                            match unsafe {
                                FramedDataMsg::parse_unchecked(msg.msginfo, &mut msg.buffer)
                            } {
                                Some(msg) => app.on_framed_data(&ctx, msg),
                                None => {
                                    app.on_unknown_message(&ctx, msg);
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

pub fn app_loop<E: DeserializeOwned, A: EventLoop<E>>(
    program_name: &'static str,
    vsyscall: *const VsyscallPage,
) {
    tls::init_thread_local(program_name);

    let env_json = unsafe {
        let ptr = (*vsyscall).environ_ptr;
        let len = (*vsyscall).environ_len;
        core::slice::from_raw_parts(ptr, len)
    };

    let startup_ch = unsafe {
        let id = (*vsyscall).startup_ch;
        if id.as_raw() == 0 {
            None
        } else {
            Some(Channel::from_handle(OwnedHandle::from_raw(id)))
        }
    };

    let env: E = serde_json::from_slice(&env_json).expect("failed to parse env");

    let poll = Poll::create().unwrap();
    let dispatcher = Dispatcher::new(poll);

    if let Some(ch) = startup_ch {
        dispatcher.add_channel(ch).unwrap();
    }

    let app = A::init(&dispatcher, env);

    loop {
        dispatcher.wait_and_dispatch(&app);
    }
}
