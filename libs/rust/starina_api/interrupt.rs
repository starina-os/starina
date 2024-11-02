//! A hardware interrupt object.
use core::fmt;

use starina_types::error::FtlError;
use starina_types::interrupt::Irq;

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
///
/// # Example
///
/// ```
/// use starina_api::interrupt::Interrupt;
/// use starina_api::types::interrupt::Irq;
///
/// // Ideally, you should get the IRQ from the device tree.
/// let irq = Irq::new(1);
///
/// // Acquire the ownership of the interrupt.
/// let interrupt = Interrupt::create(irq).unwrap();
///
/// // Register the interrupt to the mainloop.
/// let mut mainloop = Mainloop::new().unwrap();
/// mainloop
///     .add_interrupt(interrupt, Context::Interrupt)
///     .unwrap();
///
/// // Wait for interrupts in the mainloop...
/// loop {
///     match mainloop.next() {
///         Event::Interrupt { ctx: Context::Interrupt, .. } => {
///             // Handle the interrupt.
///             do_something();
///
///            // Tell the kernel that we have handled the interrupt and are
///            // ready for the next one.
///             interrupt.acknowledge().unwrap();
///         }
///         ev => {
///             warn!("unexpected event: {:?}", ev);
///         }
///     }
/// }
/// ```
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
