use core::cell::Ref;
use core::cell::RefCell;
use core::fmt;

use ftl_inlinedvec::InlinedVec;

use crate::arch;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::thread::Thread;

/// CPU identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CpuId(pub u8);

impl CpuId {
    pub const fn new(id: u8) -> CpuId {
        CpuId(id)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for CpuId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Per-CPU variables.
///
/// It's `#[repr(C)]` to guarantee the arch's `CpuVar` comes first and the
/// addresses of both `arch::CpuVar` and this `CpuVar` are the same for
/// convenience.
#[repr(C)]
pub struct CpuVar {
    pub arch: arch::CpuVar,
    pub cpu_id: CpuId,
    pub current_thread: RefCell<SharedRef<Thread>>,
    pub idle_thread: SharedRef<Thread>,
}

// SAFETY: `CpuVar` is a per-CPU storage. Will never be shared between CPUs
//         and thus won't be accessed at once.
unsafe impl Sync for CpuVar {}

pub fn current_thread() -> Ref<'static, SharedRef<Thread>> {
    arch::get_cpuvar().current_thread.borrow()
}

static CPUVARS: SpinLock<InlinedVec<CpuVar, { arch::NUM_CPUS_MAX }>> =
    SpinLock::new(InlinedVec::new());

/// Initializes Per-CPU variables for the current CPU.
pub fn percpu_init(cpu_id: CpuId) {
    // Initialize CpuVar slots until the CPU.
    let mut cpuvars = CPUVARS.lock();
    for _ in 0..=cpu_id.as_usize() {
        let idle_thread = Thread::new_idle();
        let cpuvar = CpuVar {
            arch: arch::CpuVar::new(&idle_thread),
            cpu_id,
            current_thread: RefCell::new(idle_thread.clone()),
            idle_thread,
        };

        if cpuvars.try_push(cpuvar).is_err() {
            panic!("too many CPUs");
        }
    }

    arch::set_cpuvar(&mut cpuvars[cpu_id.as_usize()] as *mut CpuVar);
}
