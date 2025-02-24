//! Process management.
use core::fmt;

use crate::handle::HandleTable;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

pub struct Process {
    handles: SpinLock<HandleTable>,
}

impl Process {
    pub const fn create() -> Process {
        Process {
            handles: SpinLock::new(HandleTable::new()),
        }
    }

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process")
    }
}

pub static KERNEL_PROCESS: SharedRef<Process> = {
    static INNER: RefCounted<Process> = RefCounted::new(Process::create());
    unsafe { SharedRef::new_static(&INNER) }
};
