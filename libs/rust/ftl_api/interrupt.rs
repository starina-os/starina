//! A hardware interrupt object.
use core::fmt;

use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::handle::OwnedHandle;
use crate::syscall;

/// A hardware interrupt object.
pub struct Interrupt {
    handle: OwnedHandle,
}

impl Interrupt {
    /// Creates a new interrupt object for the given IRQ.
    pub fn create(irq: Irq) -> Result<Interrupt, FtlError> {
        let handle = syscall::interrupt_create(irq)?;
        let interrupt = Interrupt {
            handle: OwnedHandle::from_raw(handle),
        };

        Ok(interrupt)
    }

    /// Instantiates the object from the given handle.
    pub fn from_handle(handle: OwnedHandle) -> Interrupt {
        Interrupt { handle }
    }

    /// Returns the handle.
    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    /// Acknowledges the interrupt.
    ///
    /// This tells the CPU (or the interrupt controller) that the interrupt has
    /// been handled and we are ready to receive the next one.
    pub fn acknowledge(&self) -> Result<(), FtlError> {
        syscall::interrupt_ack(self.handle().id())
    }
}

impl fmt::Debug for Interrupt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Interrupt({:?})", self.handle)
    }
}
