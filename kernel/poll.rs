use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::hash::Hash;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::poll::Readiness;

use crate::cpuvar::current_thread;
use crate::handle::AnyHandle;
use crate::handle::Handleable;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::syscall::RetVal;
use crate::thread::Thread;
use crate::thread::ThreadState;

struct UniqueQueue<T> {
    queue: VecDeque<T>,
    set: BTreeSet<T>,
}

impl<T> UniqueQueue<T> {
    pub const fn new() -> UniqueQueue<T> {
        UniqueQueue {
            queue: VecDeque::new(),
            set: BTreeSet::new(),
        }
    }
}

impl<T: Eq + Ord + Copy + Hash> UniqueQueue<T> {
    pub fn enqueue(&mut self, value: T) {
        if self.set.contains(&value) {
            return;
        }

        self.queue.push_back(value);
        self.set.insert(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        let value = self.queue.pop_front();
        if let Some(value) = &value {
            let deleted = self.set.remove(value);
            debug_assert!(deleted);
        }

        value
    }
}

pub struct Listener {
    poll: SharedRef<Poll>,
    id: HandleId,
    interests: Readiness,
}

impl Listener {
    pub fn notify(&self, readiness: Readiness) {
        let mut mutable = self.poll.mutable.lock();
        mutable.ready_handles.enqueue(self.id);

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
    ready_handles: UniqueQueue<HandleId>,
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
                ready_handles: UniqueQueue::new(),
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
        if mutable.handles.contains_key(&id) {
            return Err(ErrorCode::AlreadyExists);
        }

        // Add the listener to the listener object.
        handle.add_listener(Listener {
            poll: self.clone(),
            interests,
            id,
        })?;

        mutable.handles.insert(id, handle.clone());

        let readiness = handle.readiness()?;

        // TODO: Check if we're interested in the event.
        // Are there any waiters waiting for an event?
        if let Some(waiter) = mutable.waiters.pop_front() {
            waiter.wake();
        } else {
            // No threads are ready to receive the event. Deliver it later once
            // a thread enters the wait state.
            mutable.ready_handles.enqueue(id);
        }

        Ok(())
    }

    pub fn try_wait(self: &SharedRef<Poll>) -> Option<Result<(HandleId, Readiness), ErrorCode>> {
        let mut mutable = self.mutable.lock();

        // Check if there are any ready events.
        while let Some(id) = mutable.ready_handles.pop() {
            let Some(handle) = mutable.handles.get_mut(&id) else {
                // The handle was removed from the poll. Try the next one.
                continue;
            };

            // TODO: Check if we're interested in the event.

            return Some(match handle.readiness() {
                Ok(readiness) => Ok((id, readiness)),
                Err(err) => Err(err),
            });
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

impl Handleable for Poll {
    fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode> {
        Err(ErrorCode::NotSupported)
    }

    fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode> {
        Err(ErrorCode::NotSupported)
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        Err(ErrorCode::NotSupported)
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
