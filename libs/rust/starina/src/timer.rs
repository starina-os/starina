pub use starina_types::timer::*;

use crate::handle::HandleId;
use crate::syscall::{timer_create, timer_set};
use starina_types::error::ErrorCode;

pub struct Timer {
    handle: HandleId,
}

impl Timer {
    pub fn new() -> Result<Self, ErrorCode> {
        let handle = timer_create()?;
        Ok(Timer { handle })
    }

    pub fn set_timeout_ns(&self, timeout_ns: u64) -> Result<(), ErrorCode> {
        timer_set(self.handle, timeout_ns)
    }

    pub fn set_timeout_ms(&self, timeout_ms: u64) -> Result<(), ErrorCode> {
        self.set_timeout_ns(timeout_ms * 1_000_000)
    }

    pub fn set_timeout_us(&self, timeout_us: u64) -> Result<(), ErrorCode> {
        self.set_timeout_ns(timeout_us * 1_000)
    }

    pub fn handle(&self) -> HandleId {
        self.handle
    }
}
