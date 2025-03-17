use crate::address::DAddr;
use crate::error::ErrorCode;
use crate::folio::Folio;
use crate::handle::OwnedHandle;
use crate::syscall;

pub struct IoBus {
    handle: OwnedHandle,
}

impl IoBus {
    pub fn map(&self, daddr: Option<DAddr>, len: usize) -> Result<Folio, ErrorCode> {
        let handle = syscall::iobus_map(self.handle.id(), daddr, len)?;
        Ok(Folio::from_handle(OwnedHandle::from_raw(handle)))
    }
}
