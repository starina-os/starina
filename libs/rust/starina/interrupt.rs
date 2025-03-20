//! A hardware interrupt object.
use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
pub use starina_types::interrupt::*;

use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

/// A hardware interrupt object.
///
/// This object provides functionalities to handle hardware interrupts from devices
/// in device drivers:
///
/// - Enable interrupts by acquiring the object ([`Interrupt::create`]).
/// - Acknowledge the interrupt ([`Interrupt::acknowledge`]).
/// - Wait for interrupts in an event loop ([`Mainloop::add_interrupt`](crate::mainloop::Mainloop::add_interrupt))
pub struct Interrupt {
    handle: OwnedHandle,
}

impl Interrupt {
    /// Creates a new interrupt object for the given IRQ.
    pub fn create(irq_matcher: IrqMatcher) -> Result<Interrupt, ErrorCode> {
        let handle = syscall::interrupt_create(irq_matcher)?;
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
    pub fn acknowledge(&self) -> Result<(), ErrorCode> {
        syscall::interrupt_ack(self.handle().id())
    }
}

impl Handleable for Interrupt {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}
