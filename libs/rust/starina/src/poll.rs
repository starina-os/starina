use alloc::sync::Arc;

pub use starina_types::poll::*;

use crate::collections::HashMap;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::sync::Mutex;
use crate::syscall;

pub struct RawPoll(OwnedHandle);

impl RawPoll {
    pub fn create() -> Result<Self, ErrorCode> {
        let poll = syscall::poll_create()?;
        Ok(Self(OwnedHandle::from_raw(poll)))
    }

    pub fn add(&self, object: HandleId, interests: Readiness) -> Result<(), ErrorCode> {
        syscall::poll_add(self.0.id(), object, interests)
    }

    pub fn remove(&self, object: HandleId) -> Result<(), ErrorCode> {
        syscall::poll_remove(self.0.id(), object)
    }

    pub fn wait(&self) -> Result<(HandleId, Readiness), ErrorCode> {
        syscall::poll_wait(self.0.id())
    }

    pub fn try_wait(&self) -> Result<(HandleId, Readiness), ErrorCode> {
        syscall::poll_try_wait(self.0.id())
    }
}

impl Handleable for RawPoll {
    fn handle_id(&self) -> HandleId {
        self.0.id()
    }
}

pub struct Poll<S> {
    raw_poll: RawPoll,
    states: Mutex<HashMap<HandleId, Arc<S>>>,
}

impl<S> Poll<S> {
    pub fn new() -> Result<Self, ErrorCode> {
        let raw = RawPoll::create()?;
        Ok(Self {
            raw_poll: raw,
            states: Mutex::new(HashMap::new()),
        })
    }

    pub fn add(&self, listenee: HandleId, state: S, interests: Readiness) -> Result<(), ErrorCode> {
        // Insert the state first. The poll might wake other threads up to start
        // handling events on this object immediately.
        self.states.lock().insert(listenee, Arc::new(state));

        if let Err(err) = self.raw_poll.add(listenee, interests) {
            self.states.lock().remove(&listenee);
            return Err(err);
        }

        Ok(())
    }

    pub fn remove(&self, object: HandleId) -> Result<(), ErrorCode> {
        self.raw_poll.remove(object)?;
        self.states.lock().remove(&object);
        Ok(())
    }

    pub fn wait(&self) -> Result<(Arc<S>, Readiness), ErrorCode> {
        loop {
            let (id, readiness) = self.raw_poll.wait()?;
            let state = self.states.lock().get(&id).cloned();

            match state {
                Some(state) => {
                    return Ok((state, readiness));
                }
                None => {
                    // If the state is not found, it might have been removed
                    // after the poll was woken up. Ignore it.
                    debug_warn!("state not found for handle {:?} in poll", id);
                    continue;
                }
            }
        }
    }

    pub fn try_wait(&self) -> Result<(Arc<S>, Readiness), ErrorCode> {
        loop {
            let (id, readiness) = self.raw_poll.try_wait()?;
            let state = self.states.lock().get(&id).cloned();

            match state {
                Some(state) => {
                    return Ok((state, readiness));
                }
                None => {
                    // If the state is not found, it might have been removed
                    // after the poll was woken up. Ignore it.
                    debug_warn!("state not found for handle {:?} in poll", id);
                    continue;
                }
            }
        }
    }
}
