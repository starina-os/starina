// TODO: static assert to ensure usize == u64

/// Context of a thread.
#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct Context {
    pub sepc: usize,
    pub sstatus: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
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

    pub fn new_kernel(pc: usize, sp: usize, arg: usize) -> Thread {
        let mut sstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        }

        Thread {
            context: Context {
                sepc: pc,
                sstatus,
                sp,
                a0: arg,
                ..Default::default()
            },
        }
    }
}
