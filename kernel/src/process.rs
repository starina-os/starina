//! Process management.
use core::fmt;

use crate::handle::HandleTable;
use crate::isolation::INKERNEL_ISOLATION;
use crate::isolation::Isolation;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;

pub struct Process {
    handles: SpinLock<HandleTable>,
    isolation: SharedRef<dyn Isolation>,
}

impl Process {
    pub const fn create(isolation: SharedRef<dyn Isolation>) -> Process {
        Process {
            handles: SpinLock::new(HandleTable::new()),
            isolation,
        }
    }

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }

    pub fn isolation(&self) -> &dyn Isolation {
        &*self.isolation
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process")
    }
}

pub static KERNEL_PROCESS: spin::Lazy<SharedRef<Process>> = spin::Lazy::new(|| {
    let process = Process::create(INKERNEL_ISOLATION.clone());
    SharedRef::new(process).unwrap()
});
