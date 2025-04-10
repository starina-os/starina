pub use starina_types::poll::*;

use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

pub struct Poll(OwnedHandle);

impl Poll {
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
}

impl Handleable for Poll {
    fn handle_id(&self) -> HandleId {
        self.0.id()
    }
}
