use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::poll::Readiness;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

pub struct Listener {
    mutable: SharedRef<SpinLock<Mutable>>,
    id: HandleId,
    interests: Readiness,
}

impl Listener {
    pub fn mark_ready(&self, readiness: Readiness) {
        let mut mutable = self.mutable.lock();
        if !mutable.ready_set.contains(&self.id) {
            mutable.ready_queue.push_back(self.id);
            mutable.ready_set.insert(self.id);
        }

        // If the event is what we listen for, wake up a single thread. We haven't
        // yet encountered a case where multiple processes are listening for the
        // same object, so it's totally fine.
        //
        // IDEA: Should we wake up the most-recently-added thread? It's not fair,
        //       but it might perform better if its working set is in the cache.
        if self.interests.contains(readiness) {
            if let Some(waiter) = mutable.waiters.pop_front() {
                waiter.wake();
            }
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let mut mutable = self.mutable.lock();
        let listenee = mutable.listenees.get_mut(&self.id).unwrap();

        // Remove the listener from the listenee.
        let mut new_listeners = Vec::with_capacity(listenee.listeners.len() - 1);
        for listener in &listenee.listeners {
            if !SharedRef::ptr_eq_self(listener, self) {
                new_listeners.push(listener.clone());
            }
        }
        listenee.listeners = new_listeners;
    }
}

pub struct ListenerSet {
    listeners: Vec<SharedRef<Listener>>,
}

impl ListenerSet {
    pub fn new() -> ListenerSet {
        ListenerSet {
            listeners: Vec::new(),
        }
    }

    pub fn mark_ready(&self, readiness: Readiness) {
        for listener in &self.listeners {
            listener.mark_ready(readiness);
        }
    }

    pub fn add_listener(&mut self, listener: SharedRef<Listener>) {
        self.listeners.push(listener);
    }
}

struct Listenee {
    handle: AnyHandle,
    listeners: Vec<SharedRef<Listener>>,
}

struct Mutable {
    listenees: BTreeMap<HandleId, Listenee>,
    ready_queue: VecDeque<HandleId>,
    ready_set: BTreeSet<HandleId>,
    waiters: VecDeque<SharedRef<Thread>>,
}

pub struct Poll {
    mutable: SharedRef<SpinLock<Mutable>>,
}

impl Poll {
    pub fn new() -> SharedRef<Poll> {
        SharedRef::new(Poll {
            mutable: SharedRef::new(SpinLock::new(Mutable {
                listenees: BTreeMap::new(),
                ready_queue: VecDeque::new(),
                ready_set: BTreeSet::new(),
                waiters: VecDeque::new(),
            })),
        })
    }

    pub fn add(
        &self,
        handle: AnyHandle,
        id: HandleId,
        interests: Readiness,
    ) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.listenees.contains_key(&id) {
            return Err(ErrorCode::AlreadyExists);
        }

        let listener = SharedRef::new(Listener {
            mutable: self.mutable.clone(),
            interests,
            id,
        });

        // Add the listener to the listenee object.
        handle.add_listener(listener.clone());

        // Add the listenee to the poll.
        mutable
            .listenees
            .entry(id)
            .or_insert(Listenee {
                handle,
                listeners: Vec::new(),
            })
            .listeners
            .push(listener);

        if let Some(waiter) = mutable.waiters.pop_front() {
            waiter.wake();
        } else {
            // No threads are ready to receive the event. Deliver it later once
            // a thread enters the wait state.
            if !mutable.ready_set.contains(&id) {
                mutable.ready_queue.push_back(id);
                mutable.ready_set.insert(id);
            }
        }

        Ok(())
    }

    pub fn wait(&self) -> Result<(HandleId, Readiness), ErrorCode> {
        let mut mutable = self.mutable.lock();

        // Check if there are any ready events.
        while let Some(id) = mutable.ready_queue.pop_front() {
            let _ = mutable.ready_set.remove(&id);

            let listenee = match mutable.listenees.get_mut(&id) {
                Some(listenee) => listenee,
                None => {
                    // The listenee was removed from the poll. Try the next one.
                    continue;
                }
            };

            let readiness = listenee.handle.readiness();
            return Ok((id, readiness));
        }

        // No events are ready. Block the current thread and wait for an event.
        mutable.waiters.push_back(current_thread().clone());
        drop(mutable);
        Thread::sleep_current();
    }
}
