//! Virtual memory space management.
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::FtlError;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::boot::USERMODE_ENABLED;
use crate::folio::Folio;
use crate::handle::Handle;

pub struct VmSpace {
    arch: arch::VmSpace,
}

impl VmSpace {
    pub fn new() -> Result<VmSpace, FtlError> {
        let arch = arch::VmSpace::new()?;
        Ok(VmSpace { arch })
    }

    pub fn arch(&self) -> &arch::VmSpace {
        &self.arch
    }

    pub fn map_vaddr(
        &self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), FtlError> {
        self.arch.map_fixed(vaddr, paddr, len, prot)?;
        Ok(())
    }

    pub fn map_vaddr_user(
        &self,
        vaddr: VAddr, // TODO: should be UAddr
        paddr: PAddr,
        len: usize,
        mut prot: PageProtect,
    ) -> Result<(), FtlError> {
        if USERMODE_ENABLED {
            prot |= PageProtect::USER;
        }

        self.map_vaddr(vaddr, paddr, len, prot)?;
        Ok(())
    }

    fn map(
        &self,
        vaddr: VAddr,
        folio: Folio,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), FtlError> {
        let paddr = folio.paddr();

        // The arch's page table will own the folio.
        core::mem::forget(folio);

        self.map_vaddr(vaddr, paddr, len, prot)
    }

    pub fn map_kernel(
        &self,
        vaddr: VAddr,
        folio: Folio,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), FtlError> {
        self.map(vaddr, folio, len, prot)
    }

    pub fn map_user(
        &self,
        vaddr: VAddr,
        folio: Folio,
        len: usize,
        mut prot: PageProtect,
    ) -> Result<(), FtlError> {
        if USERMODE_ENABLED {
            prot |= PageProtect::USER;
        }

        self.map(vaddr, folio, len, prot)
    }

    pub fn map_anywhere_kernel(
        &self,
        len: usize,
        folio: Handle<Folio>,
        prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        self.map_anywhere(len, folio, prot)
    }

    pub fn map_anywhere_user(
        &self,
        len: usize,
        folio: Handle<Folio>,
        mut prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        if USERMODE_ENABLED {
            prot |= PageProtect::USER;
        }

        self.map_anywhere(len, folio, prot)
    }

    fn map_anywhere(
        &self,
        len: usize,
        folio: Handle<Folio>,
        prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        if len != folio.len() {
            return Err(FtlError::InvalidArg);
        }

        let paddr = folio.paddr();

        // The arch's page table will own the folio.
        core::mem::forget(folio);

        self.arch.map_anywhere(paddr, len, prot)
    }

    pub fn switch(&self) {
        self.arch.switch();
    }
}
