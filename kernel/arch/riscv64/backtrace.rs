use core::arch::asm;
use core::mem::size_of;

const BACKTRACE_MAX_DEPTH: usize = 16;

extern "C" {
    static __kernel_start: u8;
}

#[repr(C, packed)]
struct StackFrame {
    fp: u64,
    ra: u64,
}

pub fn backtrace<F>(mut callback: F)
where
    F: FnMut(usize),
{
    let mut fp: u64;
    let mut ra: u64;
    unsafe {
        asm!(r#"
                mv {}, fp
                mv {}, ra
            "#,
            out(reg) fp,
            out(reg) ra,
        );
    }

    for i in 0..BACKTRACE_MAX_DEPTH {
        let kernel_start = &raw const __kernel_start as u64;

        // Substract 4 because the return address is the address of the next instruction
        // after the call instruction. We want the one of the call instruction.
        ra = ra.saturating_sub(4);

        if ra < kernel_start || fp < kernel_start {
            break;
        }

        if i > 0 {
            callback(ra as usize);
        }

        unsafe {
            let frame = fp.saturating_sub(size_of::<StackFrame>() as u64) as *const StackFrame;
            fp = (*frame).fp;
            ra = (*frame).ra;
        }
    }
}
