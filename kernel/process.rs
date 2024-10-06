//! Process management.
use core::fmt;

use crate::arch::get_cpuvar;
use crate::arch::return_to_user;
use crate::handle::HandleTable;
use crate::refcount::SharedRef;
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

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }

    pub fn exit_current() -> ! {
        {
            let cpuvar = get_cpuvar();
            let mut current_thread = cpuvar.current_thread.borrow_mut();
            current_thread.set_runnable();
            *current_thread = cpuvar.idle_thread.clone();
        }

        debug_warn!("exited a process");

        // TODO: Destroy the process.
        return_to_user();
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
    &KERNEL_PROCESS
}

pub fn init() {
    // TODO: Make sure it's not accidentally dereferenced before.
    spin::Lazy::force(&KERNEL_PROCESS);
}
