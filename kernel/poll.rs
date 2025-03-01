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
use crate::thread::ThreadState;

pub struct Listener {
    poll: SharedRef<Poll>,
    id: HandleId,
    interests: Readiness,
}

impl Listener {
    pub fn notify(&self, readiness: Readiness) {
        let mut mutable = self.poll.mutable.lock();
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

pub struct ListenerSet {
    listeners: Vec<Listener>,
}

impl ListenerSet {
    pub fn new() -> ListenerSet {
        ListenerSet {
            listeners: Vec::new(),
        }
    }

    pub fn notify_all(&self, readiness: Readiness) {
        for listener in &self.listeners {
            listener.notify(readiness);
        }
    }

    pub fn add_listener(&mut self, listener: Listener) {
        self.listeners.push(listener);
    }

    pub fn remove_listener(&mut self, poll: &Poll) {
        self.listeners
            .retain(|listener| SharedRef::ptr_eq_self(&listener.poll, poll));
    }
}

struct Mutable {
    handles: BTreeMap<HandleId, AnyHandle>,
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
                handles: BTreeMap::new(),
                ready_queue: VecDeque::new(),
                ready_set: BTreeSet::new(),
                waiters: VecDeque::new(),
            })),
        })
    }

    pub fn add(
        self: &SharedRef<Poll>,
        handle: AnyHandle,
        id: HandleId,
        interests: Readiness,
    ) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.handles.try_insert(id, handle.clone()).is_err() {
            return Err(ErrorCode::AlreadyExists);
        }

        // Add the listener to the listener object.
        handle.add_listener(Listener {
            poll: self.clone(),
            interests,
            id,
        });

        // Are there any waiters waiting for an event?
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

    pub fn try_wait(self: &SharedRef<Poll>) -> Option<Result<(HandleId, Readiness), ErrorCode>> {
        let mut mutable = self.mutable.lock();

        // Check if there are any ready events.
        while let Some(id) = mutable.ready_queue.pop_front() {
            let deleted = mutable.ready_set.remove(&id);
            debug_assert!(deleted);

            let Some(handle) = mutable.handles.get_mut(&id) else {
                // The handle was removed from the poll. Try the next one.
                continue;
            };

            let readiness = handle.readiness();
            return Some(Ok((id, readiness)));
        }

        // No events are ready. Block the current thread.
        //
        // WARNING: Thread::switch will never return. Clean up all resources
        //          before calling it!
        let current_thread = current_thread();
        mutable.waiters.push_back(current_thread.clone());
        current_thread.set_state(ThreadState::BlockedByPoll(self.clone()));
        None
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        let mut mutable = self.mutable.lock();
        for waiter in mutable.waiters.drain(..) {
            todo!("wake up the waiter and let it know that the poll is closed");
            // waiter.wake(Continuation::FailedWith(ErrorCode::Closed));
        }

        for handle in mutable.handles.values() {
            handle.remove_listener(self);
        }
    }
}
