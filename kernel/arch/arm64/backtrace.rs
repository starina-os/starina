use core::arch::asm;
use core::mem::size_of;

const BACKTRACE_MAX_DEPTH: usize = 16;

extern "C" {
    static __kernel_start: u8;
}

#[repr(C, packed)]
struct StackFrame {
    fp: u64,
    lr: u64,
}

pub fn backtrace<F>(mut callback: F)
where
    F: FnMut(usize),
{
    let mut fp: u64;
    let mut lr: u64;
    unsafe {
        asm!(r#"
                mov {}, fp
                mov {}, lr
            "#,
            out(reg) fp,
            out(reg) lr,
        );
    }

    for i in 0..BACKTRACE_MAX_DEPTH {
        let kernel_start = unsafe { &__kernel_start as *const _ as u64 };

        if lr < kernel_start || fp < kernel_start {
            break;
        }

        if i > 0 {
            callback(lr as usize);
        }

        unsafe {
            let frame = fp.saturating_sub(size_of::<StackFrame>() as u64) as *const StackFrame;
            fp = (*frame).fp;
            lr = (*frame).lr;
        }
    }
}
