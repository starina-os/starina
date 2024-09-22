//! A virtual address space object.
use crate::handle::OwnedHandle;

/// A virtual address space object.
#[derive(Debug)]
pub struct VmSpace {
    handle: OwnedHandle,
}

impl VmSpace {
    /// Instantiates the object from the given handle.
    pub fn from_handle(handle: OwnedHandle) -> VmSpace {
        VmSpace { handle }
    }

    /// Returns the handle.
    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }
}
