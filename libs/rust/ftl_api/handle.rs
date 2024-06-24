use ftl_types::handle::HandleId;

use crate::syscall;

/// An owned handle, which will be closed when dropped.
///
/// # Invariant: `OwnedHandle` can be transmuted to `HandleId`
///
/// This type is marked as `#[repr(transparent)]` to ensure that it can be
/// transmuted to a `HandleId`. Some code depend on this fact so don't change
/// the sturcture of this type!
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct OwnedHandle(HandleId);

impl OwnedHandle {
    pub const fn from_raw(id: HandleId) -> OwnedHandle {
        OwnedHandle(id)
    }

    pub fn id(&self) -> HandleId {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if let Err(err) = syscall::handle_close(self.0) {
            // TODO: Closing a handle may fail, but `Drop::drop` doesn't allow
            //       returning an error. We should log this error here to notice
            //       the potential bugs.
        }
    }
}
