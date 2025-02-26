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
