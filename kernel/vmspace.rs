use alloc::vec::Vec;

use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::vmspace::PageProtect;

use crate::arch::paddr2vaddr;
use crate::folio::Folio;
use crate::handle::Handle;
use crate::ref_counted::StaticRef;
use crate::spinlock::SpinLock;

pub static KERNEL_VMSPACE: StaticRef<VmSpace> = StaticRef::new(VmSpace::kernel_space());

struct Mutable {
    folios: Vec<Handle<Folio>>,
}

pub struct VmSpace {
    kernel_space: bool,
    mutable: SpinLock<Mutable>,
}

impl VmSpace {
    pub const fn kernel_space() -> VmSpace {
        VmSpace {
            kernel_space: true,
            mutable: SpinLock::new(Mutable { folios: Vec::new() }),
        }
    }

    pub fn map(
        &self,
        len: usize,
        folio: Handle<Folio>,
        _prot: PageProtect,
    ) -> Result<VAddr, FtlError> {
        if len != folio.len() {
            return Err(FtlError::InvalidArg);
        }

        if self.kernel_space {
            let vaddr = paddr2vaddr(folio.paddr())?;
            self.mutable.lock().folios.push(folio);
            return Ok(vaddr);
        }

        unimplemented!("userspace support")
    }
}
