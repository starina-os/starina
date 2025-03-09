use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::fmt;
use core::hash::Hash;

use hashbrown::HashSet;
use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::poll::Readiness;

use crate::handle::AnyHandle;
use crate::handle::Handleable;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::syscall::BlockableSyscallResult;
use crate::thread::Thread;
use crate::thread::ThreadState;
use crate::utils::fxhashmap::FxHashMap;

struct UniqueQueue<T> {
    queue: VecDeque<T>,
    set: HashSet<T>,
}

impl<T> UniqueQueue<T> {
    pub fn new() -> UniqueQueue<T> {
        UniqueQueue {
            queue: VecDeque::new(),
            set: HashSet::new(),
        }
    }
}

impl<T: Eq + Ord + Copy + Hash> UniqueQueue<T> {
    pub fn enqueue(&mut self, value: T) -> Result<(), ErrorCode> {
        if self.set.contains(&value) {
            return Ok(());
        }

        self.queue
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;
        self.set
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;

        self.queue.push_back(value);
        self.set.insert(value);
        Ok(())
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
        if mutable.ready_handles.enqueue(self.id).is_err() {
            debug_warn!("failed to notify listener due to out-of-memory");
            return;
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

    pub fn add_listener(&mut self, listener: Listener) -> Result<(), ErrorCode> {
        self.listeners
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;

        self.listeners.push(listener);
        Ok(())
    }

    pub fn remove_listener(&mut self, poll: &Poll) {
        self.listeners
            .retain(|listener| SharedRef::ptr_eq_self(&listener.poll, poll));
    }
}

struct Listenee {
    handle: AnyHandle,
    interests: Readiness,
}

struct Mutable {
    listenee: FxHashMap<HandleId, Listenee>,
    ready_handles: UniqueQueue<HandleId>,
    waiters: VecDeque<SharedRef<Thread>>,
}

pub struct Poll {
    mutable: SharedRef<SpinLock<Mutable>>,
}

impl Poll {
    pub fn new() -> Result<SharedRef<Poll>, ErrorCode> {
        let mutable = SharedRef::new(SpinLock::new(Mutable {
            listenee: FxHashMap::new(),
            ready_handles: UniqueQueue::new(),
            waiters: VecDeque::new(),
        }))?;

        let poll = SharedRef::new(Poll { mutable })?;
        Ok(poll)
    }

    pub fn add(
        self: &SharedRef<Poll>,
        handle: AnyHandle,
        id: HandleId,
        interests: Readiness,
    ) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.listenee.contains_key(&id) {
            return Err(ErrorCode::AlreadyExists);
        }

        // Add the listener to the listener object.
        handle.add_listener(Listener {
            poll: self.clone(),
            interests,
            id,
        })?;

        mutable
            .listenee
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;

        mutable.listenee.insert(
            id,
            Listenee {
                handle: handle.clone(),
                interests,
            },
        );

        let readiness = handle.readiness()?;
        if readiness.contains(interests) {
            // Are there any waiters waiting for an event?
            if let Some(waiter) = mutable.waiters.pop_front() {
                waiter.wake();
            } else {
                // No threads are ready to receive the event. Deliver it later once
                // a thread enters the wait state.
                if mutable.ready_handles.enqueue(id).is_err() {
                    debug_warn!("failed to enqueue a ready handle due to out-of-memory");
                }
            }
        }

        Ok(())
    }

    pub fn try_wait(
        self: &SharedRef<Poll>,
        current: &SharedRef<Thread>,
    ) -> BlockableSyscallResult<(HandleId, Readiness)> {
        let mut mutable = self.mutable.lock();

        // Check if there are any ready events.
        while let Some(id) = mutable.ready_handles.pop() {
            let Some(listenee) = mutable.listenee.get_mut(&id) else {
                // The handle was removed from the poll. Try the next one.
                continue;
            };

            let readiness = match listenee.handle.readiness() {
                Ok(readiness) => readiness,
                Err(e) => {
                    debug_warn!("failed to get readiness for handle: {:?}", e);
                    return BlockableSyscallResult::Done(Err(e));
                }
            };

            let interested = listenee.interests & readiness;
            if !interested.is_empty() {
                return BlockableSyscallResult::Done(Ok((id, interested)));
            }
        }

        // No events are ready. Block the current thread.
        //
        // WARNING: Thread::switch will never return. Clean up all resources
        //          before calling it!

        if mutable.waiters.try_reserve(1).is_err() {
            return BlockableSyscallResult::Done(Err(ErrorCode::OutOfMemory));
        }

        mutable.waiters.push_back(current.clone());
        BlockableSyscallResult::Blocked(ThreadState::BlockedByPoll(self.clone()))
    }
}

impl Handleable for Poll {
    fn close(&self) {
        // Nothing to do.
    }

    fn add_listener(&self, _listener: Listener) -> Result<(), ErrorCode> {
        Err(ErrorCode::NotSupported)
    }

    fn remove_listener(&self, _poll: &Poll) -> Result<(), ErrorCode> {
        Err(ErrorCode::NotSupported)
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        Err(ErrorCode::NotSupported)
    }
}

impl fmt::Debug for Poll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Poll").finish()
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        let mut mutable = self.mutable.lock();
        for waiter in mutable.waiters.drain(..) {
            waiter.set_state(ThreadState::Runnable(Some(ErrorCode::Closed.into())));
        }

        for listenee in mutable.listenee.values_mut() {
            if let Err(err) = listenee.handle.remove_listener(self) {
                debug_warn!("failed to remove listener from handle: {:?}", err);
            }
        }
    }
}
