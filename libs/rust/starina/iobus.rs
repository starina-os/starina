use starina_types::handle::HandleId;

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

impl<'de> serde::Deserialize<'de> for IoBus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let handle_id: i32 = serde::Deserialize::deserialize(deserializer)?;
        let handle = OwnedHandle::from_raw(HandleId::from_raw(handle_id));
        Ok(IoBus { handle })
    }
}
