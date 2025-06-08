use core::time::Duration;

use starina_types::error::ErrorCode;
pub use starina_types::timer::*;

use crate::handle::HandleId;
use crate::handle::Handleable;
use crate::handle::OwnedHandle;
use crate::syscall;

pub struct Timer {
    handle: OwnedHandle,
}

impl Timer {
    pub fn new() -> Result<Self, ErrorCode> {
        let handle_id = syscall::timer_create()?;
        let handle = OwnedHandle::from_raw(handle_id);
        Ok(Timer { handle })
    }

    pub fn set_timeout(&self, after: Duration) -> Result<(), ErrorCode> {
        let duration_ns = after
            .as_nanos()
            .try_into()
            .map_err(|_| ErrorCode::InvalidArg)?;

        syscall::timer_set(self.handle.id(), duration_ns)
    }
}

impl Handleable for Timer {
    fn handle_id(&self) -> HandleId {
        self.handle.id()
    }
}

/// Get the current monotonic time since kernel boot.
pub fn now() -> MonotonicTime {
    // This should never fail.
    syscall::timer_now().unwrap()
}
