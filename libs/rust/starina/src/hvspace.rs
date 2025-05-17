use starina_types::address::GPAddr;
use starina_types::handle::HandleId;
use starina_types::vmspace::PageProtect;

use crate::error::ErrorCode;
use crate::folio::Folio;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

#[derive(Debug)]
pub struct HvSpace {
    handle: OwnedHandle,
}

impl HvSpace {
    pub fn new() -> Result<Self, ErrorCode> {
        let id = syscall::sys_hvspace_create()?;
        Ok(Self {
            handle: OwnedHandle::from_raw(id),
        })
    }

    pub fn map(
        &self,
        gpaddr: GPAddr,
        folio: &Folio,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        syscall::sys_hvspace_map(self.handle.id(), gpaddr, folio.handle_id(), len, prot)?;
        Ok(())
    }
}

impl Handleable for HvSpace {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}
