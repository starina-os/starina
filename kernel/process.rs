use core::fmt;

use crate::handle::HandleTable;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;

pub struct Process {
    handles: SpinLock<HandleTable>,
}

impl Process {
    pub fn create() -> Process {
        Process {
            handles: SpinLock::new(HandleTable::new()),
        }
    }

    pub fn in_kernel_space(&self) -> bool {
        // TODO: Support user space.
        true
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

static KERNEL_PROCESS: spin::Lazy<SharedRef<Process>> = spin::Lazy::new(|| {
    let proc = Process::create();
    SharedRef::new(proc)
});

pub fn kernel_process() -> &'static SharedRef<Process> {
    &*KERNEL_PROCESS
}

pub fn init() {
    // TODO: Make sure it's not accidentally dereferenced before.
    spin::Lazy::force(&KERNEL_PROCESS);
}
