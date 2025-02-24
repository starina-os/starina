/// A handle ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId(i32);

/// Allowed operations on a handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleRights(pub u8);
