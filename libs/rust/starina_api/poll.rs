//! A polling API, similar to Linux's `epoll`.
use starina_types::error::FtlError;
use starina_types::handle::HandleId;
use starina_types::poll::PollEvent;

use crate::handle::OwnedHandle;
use crate::syscall;

/// A polling API, similar to Linux's `epoll`.
pub struct Poll {
    handle: OwnedHandle,
}

impl Poll {
    /// Creates a new polling API.
    pub fn new() -> Result<Poll, FtlError> {
        let handle = syscall::poll_create()?;
        Ok(Poll {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    /// Creates a polling API from the given handle.
    pub fn from_handle(handle: OwnedHandle) -> Poll {
        Poll { handle }
    }

    /// Adds a handle to watch for events.
    pub fn add(&self, handle: HandleId, interests: PollEvent) -> Result<(), FtlError> {
        syscall::poll_add(self.handle.id(), handle, interests)
    }

    /// Removes a handle from the watch list.
    pub fn remove(&self, pollee: HandleId) -> Result<(), FtlError> {
        syscall::poll_remove(self.handle.id(), pollee)
    }

    /// Waits for an event. This is a blocking call.
    pub fn wait(&self) -> Result<(PollEvent, HandleId), FtlError> {
        let ret = syscall::poll_wait(self.handle.id())?;
        Ok((ret.event(), ret.handle()))
    }
}
