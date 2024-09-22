use core::arch::asm;

use super::thread::Context;
use crate::ref_counted::SharedRef;
use crate::thread::Thread;

pub struct CpuVar {
    pub(super) context: *mut Context,
    pub(super) s0_scratch: u64,
}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<Thread>) -> Self {
        Self {
            context: &idle_thread.arch().context as *const _ as *mut _,
            s0_scratch: 0,
        }
    }
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    // Load the address of the current CPU's `CpuVar` from `tp`.
    let cpuvar: *const crate::cpuvar::CpuVar;
    unsafe {
        asm!("mv {}, tp", out(reg) cpuvar);
    }
    unsafe { &*cpuvar }
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    // Store the address of the current CPU's `CpuVar` to `tp`.
    unsafe {
        asm!("mv tp, {}", in(reg) cpuvar);
    }
}
