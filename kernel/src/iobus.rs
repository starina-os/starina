use starina::address::DAddr;
use starina::address::PAddr;
use starina::error::ErrorCode;
use starina::poll::Readiness;
use starina_utils::alignment::is_aligned;

use crate::arch::PAGE_SIZE;
use crate::folio::Folio;
use crate::handle::Handleable;
use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;

pub static NOMMU_IOBUS: SharedRef<IoBus> = {
    static INNER: RefCounted<IoBus> = RefCounted::new(IoBus::NoMmu);
    unsafe { SharedRef::new_static(&INNER) }
};

/// A device memory address space.
pub enum IoBus {
    NoMmu,
}

impl IoBus {
    pub fn map(&self, daddr: Option<DAddr>, len: usize) -> Result<Folio, ErrorCode> {
        // In No MMU bus, we don't need to configure the IOMMU. Just allocate
        // a folio accordingly.
        debug_assert!(matches!(self, IoBus::NoMmu));

        if let Some(daddr) = daddr {
            let paddr = PAddr::new(daddr.as_usize());
            if !is_aligned(paddr.as_usize(), PAGE_SIZE) {
                return Err(ErrorCode::InvalidArg);
            }

            if !is_aligned(len, PAGE_SIZE) {
                return Err(ErrorCode::InvalidArg);
            }

            Folio::alloc_at(paddr, len)
        } else {
            Folio::alloc_for_device(len)
        }
    }
}

impl Handleable for IoBus {
    fn close(&self) {
        // Do nothing
    }

    fn add_listener(&self, _listener: Listener) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
    fn remove_listener(&self, _poll: &Poll) -> Result<(), ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
    fn readiness(&self) -> Result<Readiness, ErrorCode> {
        debug_warn!("unsupported method at {}:{}", file!(), line!());
        Err(ErrorCode::NotSupported)
    }
}
