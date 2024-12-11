use starina_types::{error::ErrorCode, handle::HandleId, poll::PollEvent};

use crate::{syscall, handle::OwnedHandle};

/// A polling API, similar to Linux's `epoll`.
pub struct Poll {
    handle: OwnedHandle,
}

impl Poll {
    /// Waits for an event. This is a blocking call.
    pub fn wait(&self) -> Result<(PollEvent, HandleId), ErrorCode> {
        let ret = syscall::poll_wait(self.handle.id())?;
        Ok((PollEvent::from_raw( ret.bits()), ret.id()))
    }
}
