use core::fmt;

use arrayvec::ArrayVec;

use crate::arch::set_cpuvar;
use crate::arch::{self};
use crate::spinlock::SpinLock;

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
}

static CPUVARS: SpinLock<ArrayVec<CpuVar, { arch::NUM_CPUS_MAX }>> =
    SpinLock::new(ArrayVec::new_const());

/// Initializes Per-CPU variables for the current CPU.
pub fn percpu_init(cpu_id: CpuId) {
    // Initialize CpuVar slots until the CPU.
    let mut cpuvars = CPUVARS.lock();
    for _ in 0..=cpu_id.as_usize() {
        cpuvars.push(CpuVar {
            arch: arch::CpuVar::new(),
            cpu_id,
        });
    }

    set_cpuvar(&cpuvars[cpu_id.as_usize()] as *const CpuVar);
}
