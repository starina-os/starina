use core::ops::BitOr;

use crate::error::ErrorCode;
use crate::syscall::RetVal;

/// A handle ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandleId(i32);

impl HandleId {
    /// Creates a handle ID from a raw integer.
    pub const fn from_raw(raw: i32) -> HandleId {
        HandleId(raw)
    }

    pub const fn as_raw(&self) -> i32 {
        self.0
    }

    pub fn from_raw_isize(raw: isize) -> Result<HandleId, ErrorCode> {
        if let Ok(raw) = raw.try_into() {
            Ok(HandleId(raw))
        } else {
            Err(ErrorCode::InvalidSyscall)
        }
    }
}

impl From<HandleId> for RetVal {
    fn from(handle: HandleId) -> Self {
        RetVal::new(handle.as_raw() as isize)
    }
}

/// Allowed operations on a handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleRights(pub u8);

impl HandleRights {
    pub const READ: HandleRights = HandleRights(1 << 0);
    pub const WRITE: HandleRights = HandleRights(1 << 1);
    pub const POLL: HandleRights = HandleRights(1 << 2);
    pub const MAP: HandleRights = HandleRights(1 << 3);

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
