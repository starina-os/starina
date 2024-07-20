use core::fmt;

use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::handle::OwnedHandle;
use crate::syscall;

pub struct Interrupt {
    handle: OwnedHandle,
}

impl Interrupt {
    pub fn from_handle(handle: OwnedHandle) -> Interrupt {
        Interrupt { handle }
    }

    pub fn create(irq: Irq) -> Result<Interrupt, FtlError> {
        let handle = syscall::interrupt_create(irq)?;
        let interrupt = Interrupt {
            handle: OwnedHandle::from_raw(handle),
        };

        Ok(interrupt)
    }

    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    pub fn ack(&self) -> Result<(), FtlError> {
        syscall::interrupt_ack(self.handle().id())
    }
}

impl fmt::Debug for Interrupt {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
