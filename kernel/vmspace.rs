//! Virtual memory space management.
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::vmspace::PageProtect;

use crate::arch;
use crate::folio::Folio;
use crate::handle::Handle;

pub struct VmSpace {
    arch: arch::VmSpace,
}

impl VmSpace {
    pub fn kernel_space() -> Result<VmSpace, FtlError> {
        let arch = arch::VmSpace::new()?;
        Ok(VmSpace { arch })
    }

    pub fn arch(&self) -> &arch::VmSpace {
        &self.arch
    }

    pub fn map(
        &self,
        vaddr: VAddr,
        folio: Folio,
        len: usize,
        _prot: PageProtect,
    ) -> Result<(), FtlError> {
        let paddr = folio.paddr();

        // The arch's page table will own the folio.
        core::mem::forget(folio);

        self.arch.map_fixed(vaddr, paddr, len)?;
        Ok(())
    }

    pub fn map_anywhere(
        &self,
        len: usize,
        folio: Handle<Folio>,
        _prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        if len != folio.len() {
            return Err(FtlError::InvalidArg);
        }

        let paddr = folio.paddr();

        // The arch's page table will own the folio.
        core::mem::forget(folio);

        self.arch.map_anywhere(paddr, len)
    }

    pub fn switch(&self) {
        self.arch.switch();
    }
}
