use core::arch::asm;

use super::thread::Context;
use crate::ref_counted::SharedRef;
use crate::thread::Thread;

pub struct CpuVar {
    pub(super) context: *mut Context,
}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<Thread>) -> Self {
        Self {
            context: &idle_thread.arch().context as *const _ as *mut _,
        }
    }
}

pub fn cpuvar() -> &'static crate::cpuvar::CpuVar {
    let cpuvar: *const crate::cpuvar::CpuVar;
    unsafe {
        asm!("mrs {}, tpidr_el0", out(reg) cpuvar);
    }
    unsafe { &*cpuvar }
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    unsafe {
        asm!("msr tpidr_el0, {}", in(reg) cpuvar);
    }
}
