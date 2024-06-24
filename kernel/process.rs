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

    pub fn handles(&self) -> &SpinLock<HandleTable> {
        &self.handles
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
