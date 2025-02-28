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
        mutable.ready.push_back(self.id);
        if let Some(waiter) = mutable.waiters.pop() {
            waiter.wake();
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let mut mutable = self.mutable.lock();
        let listeners = mutable.items.get_mut(&self.id).unwrap();
        listeners.retain(|l| core::ptr::eq(l, self));
    }
}

pub struct ListenerSet {
    listeners: Vec<Listener>,
}

impl ListenerSet {
    pub fn mark_ready(&self) {
        for listener in &self.listeners {
            listener.mark_ready();
        }
    }
}

struct Mutable {
    items: BTreeMap<HandleId, Vec<Listener>>,
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

        mutable
            .items
            .entry(id)
            .or_insert_with(Vec::new)
            .push(Listener {
                mutable: self.mutable.clone(),
                handle,
                id,
            });

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
