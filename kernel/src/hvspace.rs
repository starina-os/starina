//! Virtual memory space management.
use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::poll::Readiness;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::vmspace::PageProtect;

use crate::arch;
use crate::folio::Folio;
use crate::handle::Handleable;
use crate::poll::Listener;
use crate::refcount::SharedRef;

pub struct HvSpace {
    arch: arch::HvSpace,
}

impl HvSpace {
    pub fn new() -> Result<HvSpace, ErrorCode> {
        let arch = arch::HvSpace::new()?;
        Ok(HvSpace { arch })
    }

    pub fn arch(&self) -> &arch::HvSpace {
        &self.arch
    }

    pub fn map(
        &self,
        gpaddr: GPAddr,
        folio: SharedRef<Folio>,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        if folio.len() != len {
            debug_warn!("len != folio.len");
            return Err(ErrorCode::InvalidArg);
        }

        let paddr = folio.paddr();

        // The arch's page table will own a reference to the folio.
        core::mem::forget(folio);

        self.arch.map(gpaddr, paddr, len, prot)
    }
}

impl Handleable for HvSpace {
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
