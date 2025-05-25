//! Process management.
use core::fmt;

use crate::handle::HandleTable;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::vmspace::KERNEL_VMSPACE;
use crate::vmspace::VmSpace;

pub struct Process {
    vmspace: SharedRef<VmSpace>,
    handles: SpinLock<HandleTable>,
    isolation: IsolationTy,
}

impl Process {
    pub const fn create(vmspace: SharedRef<VmSpace>, isolation: IsolationTy) -> Process {
        Process {
            vmspace,
            handles: SpinLock::new(HandleTable::new()),
            isolation,
        }
    }

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }

    pub fn vmspace(&self) -> &SharedRef<VmSpace> {
        &self.vmspace
    }

    pub fn isolation(&self) -> &IsolationTy {
        &self.isolation
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process")
    }
}

pub static KERNEL_PROCESS: spin::Lazy<SharedRef<Process>> = spin::Lazy::new(|| {
    let process = Process::create(KERNEL_VMSPACE.clone());
    SharedRef::new(process).unwrap()
});
