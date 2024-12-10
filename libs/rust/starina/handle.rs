use core::fmt;

pub use starina_types::handle::HandleId;

use crate::syscall;

/// An owned handle, which will be closed when dropped.
///
/// # Invariant: `OwnedHandle` can be transmuted to `HandleId`
///
/// This type is marked as `#[repr(transparent)]` to ensure that it can be
/// transmuted to a `HandleId`. Some code depend on this fact so don't change
/// the sturcture of this type!
#[derive(PartialEq, Eq)]
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
            // Closing a handle may fail, but Drop::drop doesn't allow
            // returning an error. Log the fact here to notice the potential
            // bug.
            warn!("failed to close handle: {:?}", err);
        }
    }
}

impl fmt::Debug for OwnedHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0.as_i32())
    }
}
