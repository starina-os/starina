use alloc::vec::Vec;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::vmspace::PageProtect;

use crate::arch;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::spinlock::SpinLock;

struct Mutable {
    folios: Vec<Handle<Folio>>,
}

pub struct VmSpace {
    arch: arch::VmSpace,
    mutable: SpinLock<Mutable>,
}

impl VmSpace {
    pub fn kernel_space() -> Result<VmSpace, FtlError> {
        let arch = arch::VmSpace::new()?;
        let mutable = SpinLock::new(Mutable { folios: Vec::new() });
        Ok(VmSpace { arch, mutable })
    }

    pub fn arch(&self) -> &arch::VmSpace {
        &self.arch
    }

    pub fn map_fixed(
        &self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        _prot: PageProtect,
    ) -> Result<(), FtlError> {
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

        // FIXME: Track folio's ownership to page table
        let mut mutable = self.mutable.lock();
        mutable.folios.push(folio);

        self.arch.map_anywhere(paddr, len)
    }

    pub fn switch(&self) {
        self.arch.switch();
    }
}
