use core::arch::asm;

use super::thread::Context;
use crate::refcount::SharedRef;
use crate::thread::Thread;

pub struct CpuVar {
    pub(super) context: *mut Context,
}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<crate::thread::Thread>) -> Self {
        Self {
            context: &raw const idle_thread.arch().context as *mut _,
        }
    }
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    let cpuvar: *const crate::cpuvar::CpuVar;
    unsafe {
        asm!("rdgsbase {}", out(reg) cpuvar);
    }
    unsafe { &*cpuvar }
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    unsafe {
        asm!("wrgsbase {}", in(reg) cpuvar);
    }
}
