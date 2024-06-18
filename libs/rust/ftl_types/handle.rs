use core::num::NonZeroIsize;

/// A handle ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId(NonZeroIsize);

impl HandleId {
    pub const fn from_nonzero(id: NonZeroIsize) -> HandleId {
        Self(id)
    }
}

/// Allowed operations on a handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleRights(pub u8);

impl HandleRights {
    pub const NONE: HandleRights = HandleRights(0);
}
