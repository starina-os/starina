use core::arch::asm;

use super::thread::Context;
use crate::refcount::SharedRef;
use crate::thread::Thread;

const CPUVAR_MAGIC: u64 = 0xc12f_c12f_c12f_c12f;

pub struct CpuVar {
    pub context: *mut Context,
    magic: u64,
    pub(super) kernel_sp: u64,
    pub(super) a0_scratch: u64,
}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<Thread>) -> Self {
        unsafe extern "C" {
            static __boot_stack_top: u8;
        }

        let sp_top = &raw const __boot_stack_top as u64;
        Self {
            context: unsafe { &raw mut (*idle_thread.arch_thread_ptr()).context },
            magic: CPUVAR_MAGIC,
            kernel_sp: sp_top,
            a0_scratch: 0,
        }
    }
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    // Load the address of the current CPU's `CpuVar` from `tp`.
    let cpuvar: *const crate::cpuvar::CpuVar;
    unsafe {
        asm!("mv {}, tp", out(reg) cpuvar);
    }

    debug_assert!(cpuvar.is_null() == false);
    debug_assert!(unsafe { (*cpuvar).arch.magic } == CPUVAR_MAGIC);
    unsafe { &*cpuvar }
}

pub fn set_cpuvar(cpuvar: *const crate::cpuvar::CpuVar) {
    debug_assert!(cpuvar.is_null() == false);
    debug_assert!(unsafe { (*cpuvar).arch.magic } == CPUVAR_MAGIC);

    // Store the address of the current CPU's `CpuVar` to `tp`.
    unsafe {
        asm!("mv tp, {}", in(reg) cpuvar);
    }
}
