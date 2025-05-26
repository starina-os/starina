use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use starina_types::syscall::RetVal;

use crate::allocator::GLOBAL_ALLOCATOR;

/// Context of a thread.
#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct Context {
    pub sepc: u64,
    pub sstatus: u64,
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
}

pub struct Thread {
    pub(super) context: Context,
}

impl Thread {
    pub fn new_idle() -> Thread {
        Thread {
            context: Default::default(),
        }
    }

    pub fn new_inkernel(pc: usize, arg: usize) -> Thread {
        let stack_size = 1024 * 1024;
        let stack =
            unsafe { GLOBAL_ALLOCATOR.alloc(Layout::from_size_align(stack_size, 16).unwrap()) };
        let sp = stack as u64 + stack_size as u64;

        let mut sstatus: u64;
        unsafe {
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        }

        Thread {
            context: Context {
                sepc: pc.try_into().unwrap(),
                sstatus,
                a0: arg.try_into().unwrap(),
                sp,
                ..Default::default()
            },
        }
    }

    pub fn set_retval(&mut self, retval: RetVal) {
        self.context.a0 = retval.as_isize() as u64;
    }
}
