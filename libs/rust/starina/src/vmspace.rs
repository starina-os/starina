use starina_types::address::VAddr;
use starina_types::handle::HandleId;
pub use starina_types::vmspace::*;

use crate::error::ErrorCode;
use crate::folio::Folio;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

pub(crate) const SELF_VMSPACE: HandleId = HandleId::from_raw(0);

#[derive(Debug)]
pub struct VmSpace {
    handle: OwnedHandle,
}

impl VmSpace {
    pub fn map_anywhere_current(
        folio: &Folio,
        len: usize,
        prot: PageProtect,
    ) -> Result<VAddr, ErrorCode> {
        let vaddr = syscall::vmspace_map(
            SELF_VMSPACE,
            VAddr::new(0), /* anywhere */
            len,
            folio.handle_id(),
            0,
            prot,
        )?;
        Ok(vaddr)
    }
}

impl Handleable for VmSpace {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}
