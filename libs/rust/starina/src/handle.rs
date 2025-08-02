pub use starina_types::handle::*;

pub use crate::prelude::*;
use crate::syscall;

#[derive(Debug)]
pub struct OwnedHandle(HandleId);

impl OwnedHandle {
    pub const fn from_raw(raw: HandleId) -> Self {
        Self(raw)
    }

    pub fn id(&self) -> HandleId {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        trace!("dropping handle {:?}", self.0);
        let _ = syscall::handle_close(self.0);
    }
}

pub trait Handleable {
    fn handle_id(&self) -> HandleId;
}
