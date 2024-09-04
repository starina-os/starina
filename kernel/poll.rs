use core::fmt;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::poll::PollEvent;
use hashbrown::HashMap;

use crate::handle::AnyHandle;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Continuation;
use crate::thread::Thread;
use crate::wait_queue::WaitQueue;

pub struct Poller {
    mutable: SharedRef<SpinLock<Mutable>>,
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

    pub fn add(&self, object: &AnyHandle, object_id: HandleId, interests: PollEvent) {
        let poller = SharedRef::new(Poller {
            mutable: self.mutable.clone(),
            handle_id: object_id,
            interests,
            ready: AtomicU8::new(0),
        });

        match object {
            AnyHandle::Channel(ch) => {
                ch.add_poller(poller.clone());
            }
            AnyHandle::Interrupt(interrupt) => {
                interrupt.add_poller(poller.clone());
            }
            _ => {
                todo!(); // TODO: support other handle types
            }
        }

        let mut mutable = self.mutable.lock();
        mutable.entries.insert(object_id, poller);
    }

    pub fn wait(self: &SharedRef<Poll>, blocking: bool) -> Result<(PollEvent, HandleId), FtlError> {
        let mut mutable = self.mutable.lock();
        for entry in mutable.entries.values() {
            let raw_ready = entry.ready.swap(0, Ordering::SeqCst); // TODO: correct ordering
            let ready = PollEvent::from_raw(raw_ready);
            if ready.is_empty() {
                continue;
            }

            let handle_id = entry.handle_id;
            if ready.contains(PollEvent::CLOSED) {
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
