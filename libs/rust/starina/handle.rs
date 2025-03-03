use core::ops::BitOr;

use crate::syscall;

/// A handle ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandleId(i32);

impl HandleId {
    /// Creates a handle ID from a raw integer.
    pub const fn from_raw(raw: i32) -> HandleId {
        HandleId(raw)
    }
}

/// Allowed operations on a handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleRights(pub u8);

impl HandleRights {
    pub const READ: HandleRights = HandleRights(1 << 0);
    pub const WRITE: HandleRights = HandleRights(1 << 1);
    pub const POLL: HandleRights = HandleRights(1 << 2);

    pub fn is_capable(&self, required: HandleRights) -> bool {
        self.0 & required.0 == required.0
    }
}

impl BitOr for HandleRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        HandleRights(self.0 | rhs.0)
    }
}

#[cfg(feature = "userspace")]
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
        todo!(); // FIXME:
    }
}

#[cfg(feature = "userspace")]
pub trait Handleable {
    fn handle_id(&self) -> HandleId;
}
