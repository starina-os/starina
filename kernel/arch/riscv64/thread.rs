use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

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
}

#[naked]
pub extern "C" fn enter_kernelland(
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
                // Disable interrupts in kernel.
                csrci sstatus, 1 << 1

                csrrw tp, sscratch, tp
                ld t0, {context_offset}(tp) // Load CpuVar.arch.context

                // Save general-purpose registers.
                sd sp, {sp_offset}(t0)
                sd gp, {gp_offset}(t0)
                sd s0, {s0_offset}(t0)
                sd s1, {s1_offset}(t0)
                sd s2, {s2_offset}(t0)
                sd s3, {s3_offset}(t0)
                sd s4, {s4_offset}(t0)
                sd s5, {s5_offset}(t0)
                sd s6, {s6_offset}(t0)
                sd s7, {s7_offset}(t0)
                sd s8, {s8_offset}(t0)
                sd s9, {s9_offset}(t0)
                sd s10, {s10_offset}(t0)
                sd s11, {s11_offset}(t0)
                sd ra, {sepc_offset}(t0)

                // Save sstatus.
                csrr t1, sstatus
                sd t1, {sstatus_offset}(t0)

                // Read the original tp temporarily saved in sscratch, and
                // restore the original sscratch value.
                csrrw t1, sscratch, tp
                sd t1, {tp_offset}(t0)

                ld sp, {kernel_sp_offset}(tp)

                // Handle the system call.
                mv a0, t0
                j {syscall_handler}
            "#,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            kernel_sp_offset = const offset_of!(crate::arch::CpuVar, kernel_sp),
            sepc_offset = const offset_of!(Context, sepc),
            sstatus_offset = const offset_of!(Context, sstatus),
            sp_offset = const offset_of!(Context, sp),
            gp_offset = const offset_of!(Context, gp),
            tp_offset = const offset_of!(Context, tp),
            s0_offset = const offset_of!(Context, s0),
            s1_offset = const offset_of!(Context, s1),
            s2_offset = const offset_of!(Context, s2),
            s3_offset = const offset_of!(Context, s3),
            s4_offset = const offset_of!(Context, s4),
            s5_offset = const offset_of!(Context, s5),
            s6_offset = const offset_of!(Context, s6),
            s7_offset = const offset_of!(Context, s7),
            s8_offset = const offset_of!(Context, s8),
            s9_offset = const offset_of!(Context, s9),
            s10_offset = const offset_of!(Context, s10),
            s11_offset = const offset_of!(Context, s11),


            syscall_handler = sym syscall_handler_trampoline,
            // syscall_handler = sym crate::syscall::syscall_handler,
        )
    }
}

#[unsafe(no_mangle)]
fn syscall_handler_trampoline(context: *mut Context) {
    crate::syscall::syscall_handler(0);
}

pub fn enter_userland(thread: *mut crate::arch::Thread) -> ! {
    let context: *mut Context = unsafe { &mut (*thread).context as *mut _ };
    let mut sstatus: u64;
    unsafe {
        asm!("csrr {0}, sstatus", out(reg) sstatus);
        sstatus |= 1 << 8; // Set SPP to go back to kernel mode
        asm!("csrw sstatus, {0}", in(reg) sstatus);
    }

    unsafe {
        asm!(r#"
            sd a0, {context_offset}(tp) // Update CpuVar.arch.context

            ld a1, {sepc_offset}(a0)
            csrw sepc, a1
            ld a1, {sstatus_offset}(a0)

            // Restore general-purpose registers.
            ld ra, {ra_offset}(a0)
            ld sp, {sp_offset}(a0)
            ld gp, {gp_offset}(a0)
            ld tp, {tp_offset}(a0)
            ld t0, {t0_offset}(a0)
            ld t1, {t1_offset}(a0)
            ld t2, {t2_offset}(a0)
            ld s0, {s0_offset}(a0)
            ld s1, {s1_offset}(a0)
            ld a1, {a1_offset}(a0)
            ld a2, {a2_offset}(a0)
            ld a3, {a3_offset}(a0)
            ld a4, {a4_offset}(a0)
            ld a5, {a5_offset}(a0)
            ld a6, {a6_offset}(a0)
            ld a7, {a7_offset}(a0)
            ld s2, {s2_offset}(a0)
            ld s3, {s3_offset}(a0)
            ld s4, {s4_offset}(a0)
            ld s5, {s5_offset}(a0)
            ld s6, {s6_offset}(a0)
            ld s7, {s7_offset}(a0)
            ld s8, {s8_offset}(a0)
            ld s9, {s9_offset}(a0)
            ld s10, {s10_offset}(a0)
            ld s11, {s11_offset}(a0)
            ld t3, {t3_offset}(a0)
            ld t4, {t4_offset}(a0)
            ld t5, {t5_offset}(a0)
            ld t6, {t6_offset}(a0)

            ld a0, {a0_offset}(a0)
            sret
        "#,
            in ("a0") context as usize,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            sepc_offset = const offset_of!(Context, sepc),
            sstatus_offset = const offset_of!(Context, sstatus),
            ra_offset = const offset_of!(Context, ra),
            sp_offset = const offset_of!(Context, sp),
            gp_offset = const offset_of!(Context, gp),
            tp_offset = const offset_of!(Context, tp),
            t0_offset = const offset_of!(Context, t0),
            t1_offset = const offset_of!(Context, t1),
            t2_offset = const offset_of!(Context, t2),
            s0_offset = const offset_of!(Context, s0),
            s1_offset = const offset_of!(Context, s1),
            a0_offset = const offset_of!(Context, a0),
            a1_offset = const offset_of!(Context, a1),
            a2_offset = const offset_of!(Context, a2),
            a3_offset = const offset_of!(Context, a3),
            a4_offset = const offset_of!(Context, a4),
            a5_offset = const offset_of!(Context, a5),
            a6_offset = const offset_of!(Context, a6),
            a7_offset = const offset_of!(Context, a7),
            s2_offset = const offset_of!(Context, s2),
            s3_offset = const offset_of!(Context, s3),
            s4_offset = const offset_of!(Context, s4),
            s5_offset = const offset_of!(Context, s5),
            s6_offset = const offset_of!(Context, s6),
            s7_offset = const offset_of!(Context, s7),
            s8_offset = const offset_of!(Context, s8),
            s9_offset = const offset_of!(Context, s9),
            s10_offset = const offset_of!(Context, s10),
            s11_offset = const offset_of!(Context, s11),
            t3_offset = const offset_of!(Context, t3),
            t4_offset = const offset_of!(Context, t4),
            t5_offset = const offset_of!(Context, t5),
            t6_offset = const offset_of!(Context, t6),
            options(noreturn)
        );
    }
}
