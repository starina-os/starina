//! Process management.
use core::fmt;

use crate::arch::get_cpuvar;
use crate::arch::return_to_user;
use crate::handle::HandleTable;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::vmspace::VmSpace;

pub struct Process {
    handles: SpinLock<HandleTable>,
    vmspace: SharedRef<VmSpace>,
}

impl Process {
    pub fn create(vmspace: SharedRef<VmSpace>) -> Process {
        Process {
            handles: SpinLock::new(HandleTable::new()),
            vmspace,
        }
    }

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
    }

    pub fn vmspace(&self) -> &SharedRef<VmSpace> {
        &self.vmspace
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

pub static KERNEL_VMSPACE: spin::Lazy<SharedRef<VmSpace>> = spin::Lazy::new(|| {
    let vmspace = VmSpace::kernel_space().unwrap();
    SharedRef::new(vmspace)
});

static KERNEL_PROCESS: spin::Lazy<SharedRef<Process>> = spin::Lazy::new(|| {
    let proc = Process::create(KERNEL_VMSPACE.clone());
    SharedRef::new(proc)
});

pub fn kernel_process() -> &'static SharedRef<Process> {
    &KERNEL_PROCESS
}

pub fn init() {
    // TODO: Make sure it's not accidentally dereferenced before.
    spin::Lazy::force(&KERNEL_PROCESS);
}
