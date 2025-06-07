use core::fmt;
use core::num::Wrapping;

use starina::timer::MonotonicTime;

use crate::handle::Handleable;
use crate::spinlock::SpinLock;

struct Mutable {
    expires_at: Option<MonotonicTime>,
}

pub struct Timer {
    mutable: SpinLock<Mutable>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            mutable: SpinLock::new(Mutable { expires_at: None }),
        }
    }
}

impl Handleable for Timer {
    fn close(&self) {
        todo!()
    }

    fn add_listener(
        &self,
        listener: crate::poll::Listener,
    ) -> Result<(), starina::error::ErrorCode> {
        todo!()
    }

    fn remove_listener(&self, poll: &crate::poll::Poll) -> Result<(), starina::error::ErrorCode> {
        todo!()
    }

    fn readiness(&self) -> Result<starina::poll::Readiness, starina::error::ErrorCode> {
        todo!()
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timer")
    }
}
