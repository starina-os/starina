//! Process management.
use core::fmt;

use crate::handle::HandleTable;
use crate::isolation::Isolation;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

pub struct Process {
    handles: SpinLock<HandleTable>,
    isolation: Isolation,
}

impl Process {
    pub const fn create(isolation: Isolation) -> Process {
        Process {
            handles: SpinLock::new(HandleTable::new()),
            isolation,
        }
    }

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }

    pub fn isolation(&self) -> &Isolation {
        &self.isolation
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process")
    }
}

pub static KERNEL_PROCESS: SharedRef<Process> = {
    static INNER: RefCounted<Process> = RefCounted::new(Process::create(Isolation::InKernel));
    unsafe { SharedRef::new_static(&INNER) }
};
