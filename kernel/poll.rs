use alloc::collections::btree_map::BTreeMap;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;

use starina::error::ErrorCode;
use starina::handle::HandleId;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

pub struct Listener {
    mutable: SharedRef<SpinLock<Mutable>>,
    id: HandleId,
    handle: AnyHandle,
}

impl Listener {
    pub fn mark_ready(&self) {
        let mut mutable = self.mutable.lock();
        if let Some(waiter) = mutable.waiters.pop() {
            waiter.wake();
        } else {
            mutable.ready.push_back(self.id);
        }
    }
}

struct Mutable {
    items: BTreeMap<HandleId, Listener>,
    ready: VecDeque<HandleId>,
    waiters: Vec<SharedRef<Thread>>,
}

pub struct Poll {
    mutable: SharedRef<SpinLock<Mutable>>,
}

impl Poll {
    pub fn add(&self, handle: AnyHandle, id: HandleId) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.items.contains_key(&id) {
            return Err(ErrorCode::AlreadyExists);
        }

        mutable.items.insert(
            id,
            Listener {
                mutable: self.mutable.clone(),
                handle,
                id,
            },
        );

        if let Some(waiter) = mutable.waiters.pop() {
            waiter.wake();
        } else {
            // No threads are ready to receive the event. Deliver it later once
            // a thread enters the wait state.
            mutable.ready.push_back(id);
        }

        Ok(())
    }

    pub fn wait(&self) -> Result<HandleId, ErrorCode> {
        let mut mutable = self.mutable.lock();
        if let Some(id) = mutable.ready.pop_front() {
            return Ok(id);
        }

        mutable.waiters.push(current_thread().clone());
        drop(mutable);
        Thread::sleep_current();
    }
}
