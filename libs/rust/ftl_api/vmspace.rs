use crate::handle::OwnedHandle;

#[derive(Debug)]
pub struct VmSpace {
    handle: OwnedHandle,
}

impl VmSpace {
    pub fn from_handle(handle: OwnedHandle) -> VmSpace {
        VmSpace { handle }
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }
}
