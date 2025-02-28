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
        listeners.retain(|l| !SharedRef::ptr_eq_self(l, self));
    }
}

pub struct ListenerSet {
    listeners: Vec<SharedRef<Listener>>,
}

impl ListenerSet {
    pub fn mark_ready(&self) {
        for listener in &self.listeners {
            listener.mark_ready();
        }
    }

    pub fn add_listener(&mut self, listener: SharedRef<Listener>) {
        self.listeners.push(listener);
    }
}

struct Mutable {
    items: BTreeMap<HandleId, Vec<SharedRef<Listener>>>,
    ready: VecDeque<HandleId>,
    waiters: Vec<SharedRef<Thread>>,
}

pub struct Poll {
    mutable: SharedRef<SpinLock<Mutable>>,
}

impl Poll {
    pub fn new() -> SharedRef<Poll> {
        SharedRef::new(Poll {
            mutable: SharedRef::new(SpinLock::new(Mutable {
                items: BTreeMap::new(),
                ready: VecDeque::new(),
                waiters: Vec::new(),
            })),
        })
    }

    pub fn add(&self, handle: AnyHandle, id: HandleId) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.items.contains_key(&id) {
            return Err(ErrorCode::AlreadyExists);
        }

        let listener = SharedRef::new(Listener {
            mutable: self.mutable.clone(),
            id,
        });

        handle.listeners_mut().add_listener(listener.clone());

        mutable
            .items
            .entry(id)
            .or_insert_with(Vec::new)
            .push(listener);

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
