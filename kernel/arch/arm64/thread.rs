use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

#[no_mangle]
extern "C" fn arm64_handle_exception() {
    panic!("unhandled exception");
}

#[no_mangle]
// #[naked]
unsafe extern "C" fn arm64_handle_interrupt() {
    todo!()
}

/// The entry point for the system call from in-kernel apps.
#[naked]
pub extern "C" fn kernel_syscall_entry(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
) -> isize {
    unsafe {
        naked_asm!(
            r#"
                // Save x0
                str x0, [sp, #-8]!

                mrs x0, tpidr_el1               // x0 = cpuvar
                ldr x0, [x0, {context_offset}]  // x0 = cpuvar->context

                str lr, [x0, {elr_offset}]
                stp  x1,  x2,  [x0, {x1_offset}]
                stp  x3,  x4,  [x0, {x3_offset}]
                stp  x5,  x6,  [x0, {x5_offset}]
                stp  x7,  x8,  [x0, {x7_offset}]
                stp  x9,  x10, [x0, {x9_offset}]
                stp  x11, x12, [x0, {x11_offset}]
                stp  x13, x14, [x0, {x13_offset}]
                stp  x15, x16, [x0, {x15_offset}]
                stp  x17, x18, [x0, {x17_offset}]
                stp  x19, x20, [x0, {x19_offset}]
                stp  x21, x22, [x0, {x21_offset}]
                stp  x23, x24, [x0, {x23_offset}]
                stp  x25, x26, [x0, {x25_offset}]
                stp  x27, x28, [x0, {x27_offset}]
                stp  x29, x30, [x0, {x29_offset}]

                // Restore x0
                mov x9, x0
                ldr x0, [sp], #8

                // Save sp
                mov x10, sp
                str x10, [x9, {sp_offset}]

                // Get the per-CPU stack
                mrs x9, tpidr_el1
                ldr x10, [x9, {kernel_sp_offset}]
                mov sp, x10

                // Handle the system call.
                bl {syscall_handler}

                // Save the return value in the thread context, and switch
                // to the next thread.
                mrs x1, tpidr_el1
                ldr x1, [x1, {context_offset}]
                str x0, [x1, {x0_offset}]
                b {switch_thread}
            "#,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            kernel_sp_offset = const offset_of!(crate::arch::CpuVar, kernel_sp),
            x0_offset = const offset_of!(Context, x0),
            x1_offset = const offset_of!(Context, x1),
            x3_offset = const offset_of!(Context, x3),
            x5_offset = const offset_of!(Context, x5),
            x7_offset = const offset_of!(Context, x7),
            x9_offset = const offset_of!(Context, x9),
            x11_offset = const offset_of!(Context, x11),
            x13_offset = const offset_of!(Context, x13),
            x15_offset = const offset_of!(Context, x15),
            x17_offset = const offset_of!(Context, x17),
            x19_offset = const offset_of!(Context, x19),
            x21_offset = const offset_of!(Context, x21),
            x23_offset = const offset_of!(Context, x23),
            x25_offset = const offset_of!(Context, x25),
            x27_offset = const offset_of!(Context, x27),
            x29_offset = const offset_of!(Context, x29),
            sp_offset = const offset_of!(Context, sp),
            elr_offset = const offset_of!(Context, elr),
            syscall_handler = sym crate::syscall::syscall_handler,
            switch_thread = sym crate::thread::switch_thread,
        );
    }
}

pub fn return_to_user(thread: *mut super::Thread, sysret: Option<isize>) -> ! {
    let context: *mut Context = unsafe { &mut (*thread).context as *mut _ };
    if let Some(value) = sysret {
        unsafe {
            (*context).x0 = value as usize;
        }
    }

    // FIXME: Save and restore spsr_el1
    let mut spsr: u64 = 0;

    // Disable interrupts - we receive only in the idle loop.
    // TODO: Allow interrupts in user mode.
    spsr |= 1 << 7;

    if crate::boot::USERMODE_ENABLED {
        todo!("update spsr");
    } else {
        spsr |= 0b0101; // EL1h
    }

    unsafe {
        asm!("msr spsr_el1, {}", in(reg) spsr);
    }

    unsafe {
        asm!(r#"
            mrs x1, tpidr_el1
            str x0, [x1, {context_offset}] // Update CpuVar.arch.context

            // Restore elr_el1
            ldr  x1, [x0, {elr_offset}]
            msr  elr_el1, x1

            // Restore sp
            ldr  x1, [x0, {sp_offset}]
            mov  sp, x1

            ldp  x29, x30, [x0, {x29_offset}]
            ldp  x27, x28, [x0, {x27_offset}]
            ldp  x25, x26, [x0, {x25_offset}]
            ldp  x23, x24, [x0, {x23_offset}]
            ldp  x21, x22, [x0, {x21_offset}]
            ldp  x19, x20, [x0, {x19_offset}]
            ldp  x17, x18, [x0, {x17_offset}]
            ldp  x15, x16, [x0, {x15_offset}]
            ldp  x13, x14, [x0, {x13_offset}]
            ldp  x11, x12, [x0, {x11_offset}]
            ldp  x9,  x10, [x0, {x9_offset}]
            ldp  x7,  x8,  [x0, {x7_offset}]
            ldp  x5,  x6,  [x0, {x5_offset}]
            ldp  x3,  x4,  [x0, {x3_offset}]
            ldp  x1,  x2,  [x0, {x1_offset}]
            ldr  x0, [x0, {x0_offset}]

            eret
        "#,
            in ("x0") context as usize,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            sp_offset = const offset_of!(Context, sp),
            elr_offset = const offset_of!(Context, elr),
            x29_offset = const offset_of!(Context, x29),
            x27_offset = const offset_of!(Context, x27),
            x25_offset = const offset_of!(Context, x25),
            x23_offset = const offset_of!(Context, x23),
            x21_offset = const offset_of!(Context, x21),
            x19_offset = const offset_of!(Context, x19),
            x17_offset = const offset_of!(Context, x17),
            x15_offset = const offset_of!(Context, x15),
            x13_offset = const offset_of!(Context, x13),
            x11_offset = const offset_of!(Context, x11),
            x9_offset = const offset_of!(Context, x9),
            x7_offset = const offset_of!(Context, x7),
            x5_offset = const offset_of!(Context, x5),
            x3_offset = const offset_of!(Context, x3),
            x1_offset = const offset_of!(Context, x1),
            x0_offset = const offset_of!(Context, x0),
            options(noreturn)
        );
    }
}

/// Context of a thread.
///
/// Only callee-saved registers are stored because caller-saved registers are
/// already saved on the stack.
#[derive(Debug, Default)]
#[repr(C)]
pub struct Context {
    x0: usize,
    x1: usize,
    x2: usize,
    x3: usize,
    x4: usize,
    x5: usize,
    x6: usize,
    x7: usize,
    x8: usize,
    x9: usize,
    x10: usize,
    x11: usize,
    x12: usize,
    x13: usize,
    x14: usize,
    x15: usize,
    x16: usize,
    x17: usize,
    x18: usize,
    x19: usize,
    x20: usize,
    x21: usize,
    x22: usize,
    x23: usize,
    x24: usize,
    x25: usize,
    x26: usize,
    x27: usize,
    x28: usize,
    x29: usize, // aka fp
    lr: usize,  // aka x30
    sp: usize,
    spsr: usize,
    elr: usize,
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

    pub fn new_kernel(pc: usize, arg: usize) -> Thread {
        Thread {
            context: Context {
                elr: pc,
                x0: arg,
                ..Default::default()
            },
        }
    }
}
