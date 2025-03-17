//! Process management.
use core::fmt;

use crate::handle::HandleTable;
use crate::isolation::Isolation;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::vmspace::KERNEL_VMSPACE;

pub struct Process {
    vmspace: SharedRef<VmSpace>,
    handles: SpinLock<HandleTable>,
    isolation: Isolation,
}

impl Process {
    pub const fn create(vmspace: SharedRef<VmSpace>, isolation: Isolation) -> Process {
        Process {
            vmspace,
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

pub static KERNEL_PROCESS: spin::Lazy<SharedRef<Process>> = spin::Lazy::new(|| {
    let process = Process::create(KERNEL_VMSPACE.clone(), Isolation::InKernel);
    SharedRef::new(&process)
});
