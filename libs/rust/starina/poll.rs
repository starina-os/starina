use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::poll::PollEvent;

use crate::handle::OwnedHandle;
use crate::syscall;

/// A polling API, similar to Linux's `epoll`.
pub struct Poll {
    handle: OwnedHandle,
}

impl Poll {
    /// Creates a new poll object.
    pub fn new() -> Result<Poll, ErrorCode> {
        let handle = syscall::poll_create()?;
        Ok(Poll {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    /// Waits for an event. This is a blocking call.
    pub fn wait(&self) -> Result<(PollEvent, HandleId), ErrorCode> {
        let ret = syscall::poll_wait(self.handle.id())?;
        Ok((PollEvent::from_raw(ret.bits()), ret.id()))
    }
}
