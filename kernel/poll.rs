use core::fmt;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

use hashbrown::HashMap;
use starina_types::error::FtlError;
use starina_types::handle::HandleId;
use starina_types::poll::PollEvent;

use crate::handle::AnyHandle;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Continuation;
use crate::thread::Thread;
use crate::wait_queue::WaitQueue;

pub struct Poller {
    mutable: SharedRef<SpinLock<Mutable>>,
    object: AnyHandle,
    handle_id: HandleId,
    interests: PollEvent,
    ready: AtomicU8,
}

impl Poller {
    pub fn set_ready(&self, ready: PollEvent) {
        let intersects = ready & self.interests;
        if intersects.is_empty() {
            return;
        }

        self.ready.fetch_or(intersects.as_raw(), Ordering::SeqCst); // TODO: correct ordering

        let mut mutable = self.mutable.lock();
        mutable.wait_queue.wake_all();
    }

    pub fn attach(self: &SharedRef<Poller>) -> Result<(), FtlError> {
        match &self.object {
            AnyHandle::Channel(ch) => {
                ch.add_poller(self.clone());
            }
            AnyHandle::Interrupt(interrupt) => {
                interrupt.add_poller(self.clone());
            }
            _ => {
                return Err(FtlError::NotSupported);
            }
        }

        Ok(())
    }

    pub fn detach(self: &SharedRef<Poller>) {
        match &self.object {
            AnyHandle::Channel(ch) => {
                ch.remove_poller(self);
            }
            AnyHandle::Interrupt(interrupt) => {
                interrupt.remove_poller(self);
            }
            _ => {
                unreachable!();
            }
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.set_ready(PollEvent::CLOSED);
    }
}

struct Mutable {
    entries: HashMap<HandleId, SharedRef<Poller>>,
    wait_queue: WaitQueue,
}

pub struct Poll {
    mutable: SharedRef<SpinLock<Mutable>>,
}

impl Poll {
    pub fn new() -> SharedRef<Poll> {
        let poll = Poll {
            mutable: SharedRef::new(SpinLock::new(Mutable {
                entries: HashMap::new(),
                wait_queue: WaitQueue::new(),
            })),
        };

        SharedRef::new(poll)
    }

    pub fn add(
        &self,
        object: &AnyHandle,
        object_id: HandleId,
        interests: PollEvent,
    ) -> Result<(), FtlError> {
        let poller = SharedRef::new(Poller {
            mutable: self.mutable.clone(),
            object: object.clone(),
            handle_id: object_id,
            interests,
            ready: AtomicU8::new(0),
        });

        poller.attach()?;

        let mut mutable = self.mutable.lock();
        mutable.entries.insert(object_id, poller);
        Ok(())
    }

    pub fn remove(&self, handle_id: HandleId) -> Result<(), FtlError> {
        let mut mutable = self.mutable.lock();
        let poller = match mutable.entries.remove(&handle_id) {
            Some(poller) => poller,
            None => {
                return Err(FtlError::HandleNotFound);
            }
        };

        poller.detach();
        Ok(())
    }

    pub fn wait(self: &SharedRef<Poll>, blocking: bool) -> Result<(PollEvent, HandleId), FtlError> {
        let mut mutable = self.mutable.lock();
        for poller in mutable.entries.values() {
            let raw_ready = poller.ready.swap(0, Ordering::SeqCst); // TODO: correct ordering
            let ready = PollEvent::from_raw(raw_ready);
            if ready.is_empty() {
                continue;
            }

            let handle_id = poller.handle_id;
            if ready.contains(PollEvent::CLOSED) {
                poller.detach();
                mutable.entries.remove(&handle_id);
            }

            return Ok((ready, handle_id));
        }

        if blocking {
            mutable.wait_queue.listen();
            drop(mutable);
            Thread::block_current(Continuation::PollWait { poll: self.clone() });
        }

        Err(FtlError::WouldBlock)
    }
}

impl fmt::Debug for Poll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Poll")
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        let mut mutable = self.mutable.lock();
        for (_, poller) in mutable.entries.drain() {
            poller.detach();
        }
    }
}
