//! Virtual memory space management.
use starina::error::ErrorCode;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::folio::Folio;
use crate::handle::Handleable;
use crate::refcount::SharedRef;

pub static KERNEL_VMSPACE: spin::Lazy<SharedRef<VmSpace>> = spin::Lazy::new(|| {
    let vmspace = VmSpace::new().expect("failed to create kernel vmspace");
    SharedRef::new(&vmspace)
});

pub struct VmSpace {
    arch: arch::VmSpace,
}

impl VmSpace {
    pub fn new() -> Result<VmSpace, ErrorCode> {
        let arch = arch::VmSpace::new()?;
        Ok(VmSpace { arch })
    }

    pub fn arch(&self) -> &arch::VmSpace {
        &self.arch
    }

    pub fn map_anywhere(
        &self,
        folio: SharedRef<Folio>,
        prot: PageProtect,
    ) -> Result<VAddr, ErrorCode> {
        let paddr = folio.paddr();

        // The arch's page table will own an reference to the folio.
        core::mem::forget(folio);

        self.arch.map_anywhere(paddr, folio.len(), prot)
    }

    pub fn switch(&self) {
        self.arch.switch();
    }
}

impl Handleable for VmSpace {
    fn close(&self) {
        // Do nothing
    }

    fn add_listener(&self, _listener: Listener) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn remove_listener(&self, _poll: &crate::poll::Poll) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }

    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
}
