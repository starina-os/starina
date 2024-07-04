use core::fmt;

use ftl_types::error::FtlError;
use ftl_types::signal::SignalBits;

use crate::handle::OwnedHandle;
use crate::syscall;

pub struct Signal {
    handle: OwnedHandle,
}

impl Signal {
    pub fn from_handle(handle: OwnedHandle) -> Signal {
        Signal { handle }
    }

    pub fn create() -> Result<Signal, FtlError> {
        let handle = syscall::signal_create()?;

        Ok(Signal {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    pub fn update(&self, value: SignalBits) -> Result<(), FtlError> {
        syscall::signal_update(self.handle.id(), value)
    }

    pub fn clear(&self) -> Result<SignalBits, FtlError> {
        syscall::signal_clear(self.handle.id())
    }
}

impl fmt::Debug for Signal {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
