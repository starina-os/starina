use core::arch::asm;

use super::thread::Context;
use crate::refcount::SharedRef;
use crate::thread::Thread;

pub struct CpuVar {
    pub(super) context: *mut Context,
    pub(super) kernel_sp: u64,
}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<Thread>) -> Self {
        extern "C" {
            static __boot_stack_top: u8;
        }

        let sp_top = &raw const __boot_stack_top as u64;
        Self {
            context: &idle_thread.arch().context as *const _ as *mut _,
            kernel_sp: sp_top,
        }
    }
}

pub fn cpuvar() -> &'static crate::cpuvar::CpuVar {
    let cpuvar: *const crate::cpuvar::CpuVar;
    unsafe {
        asm!("mrs {}, tpidr_el1", out(reg) cpuvar);
    }
    unsafe { &*cpuvar }
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    unsafe {
        asm!("msr tpidr_el1, {}", in(reg) cpuvar);
    }
}
