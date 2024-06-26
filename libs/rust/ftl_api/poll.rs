use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::poll::PollEvent;

use crate::handle::OwnedHandle;
use crate::syscall;

pub struct Poll {
    handle: OwnedHandle,
}

impl Poll {
    pub fn from_handle(handle: OwnedHandle) -> Poll {
        Poll { handle }
    }

    pub fn add(&self, handle: HandleId, interests: PollEvent) -> Result<(), FtlError> {
        syscall::poll_add(self.handle.id(), handle, interests)
    }

    pub fn wait(&self) -> Result<(PollEvent, HandleId), FtlError> {
        let ret = syscall::poll_wait(self.handle.id())?;
        Ok((ret.event(), ret.handle()))
    }
}
