//! A semaphore-like API.
use core::fmt;

use ftl_types::error::FtlError;
use ftl_types::signal::SignalBits;

use crate::handle::OwnedHandle;
use crate::syscall;

/// A semaphore-like asynchronous inter-process communication primitive.
///
/// Each signal has 32-bit-wide bitfields. Updating the bitfields wakes up
/// waiting processes. You can update the bitfields multiple times before
/// it is cleared, but it can't tell how many times it was updated. In other
/// words, it guarantees bitfields are set at least once.
///
/// # [`Signal`] vs. [`Channel`](crate::channel::Channel)
///
/// While [`Signal`] cannot carry data more than bitfields,
/// it is more lightweight and faster than [`Channel`](crate::channel::Channel).
///
/// Use [`Signal`] when you need to notify other processes of an event, especially when you just want to wake up a sleeping process.
pub struct Signal {
    handle: OwnedHandle,
}

impl Signal {
    /// Creates a new signal.
    pub fn create() -> Result<Signal, FtlError> {
        let handle = syscall::signal_create()?;

        Ok(Signal {
            handle: OwnedHandle::from_raw(handle),
        })
    }

    /// Instantiates the object from the given handle.
    pub fn from_handle(handle: OwnedHandle) -> Signal {
        Signal { handle }
    }

    /// Returns the handle.
    pub fn handle(&self) -> &OwnedHandle {
        &self.handle
    }

    /// Updates the signal. Non-blocking.
    ///
    /// This bit-wise ORs the given value to the signal, and wakes up waiting
    /// processes if any.
    pub fn update(&self, value: SignalBits) -> Result<(), FtlError> {
        syscall::signal_update(self.handle.id(), value)
    }

    /// Clears the signal, and returns the previous value. Non-blocking.
    pub fn clear(&self) -> Result<SignalBits, FtlError> {
        syscall::signal_clear(self.handle.id())
    }
}

impl fmt::Debug for Signal {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
