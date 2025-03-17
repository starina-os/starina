use crate::address::DAddr;
use crate::error::ErrorCode;
use crate::handle::HandleId;
use crate::handle::OwnedHandle;
use crate::syscall;

pub struct IoBus {
    handle: OwnedHandle,
}

impl IoBus {
    pub fn map(&self, vm: HandleId, daddr: Option<DAddr>, len: usize) -> Result<DAddr, ErrorCode> {
        todo!()
    }
}
