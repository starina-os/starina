use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use super::interrupt::interrupt_handler;
use super::thread::Context;

/// The entry point for the system call from in-kernel apps.
#[naked]
pub unsafe extern "C" fn kernel_syscall_entry(
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
                // TODO: Perhaps it is *always* disabled?
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

                // Save t0 temporarily as it will be used later.
                mv s0, t0

                // Handle the system call.
                call {syscall_handler}

                // Save the return value in the thread context, and switch
                // to the next thread.
                sd a0, {a0_offset}(s0)
                j {switch_thread}
            "#,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            sepc_offset = const offset_of!(Context, sepc),
            sstatus_offset = const offset_of!(Context, sstatus),
            sp_offset = const offset_of!(Context, sp),
            gp_offset = const offset_of!(Context, gp),
            tp_offset = const offset_of!(Context, tp),
            a0_offset = const offset_of!(Context, a0),
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
            syscall_handler = sym crate::syscall::syscall_handler,
            switch_thread = sym crate::thread::switch_thread,
        )
    }
}

/// The entry point for traps: exceptions, interrupts, and system calls.
#[link_section = ".text.switch_to_kernel"]
#[naked]
pub unsafe extern "C" fn switch_to_kernel() -> ! {
    unsafe {
        naked_asm!(
            r#"
                csrrw tp, sscratch, tp      // Save tp to sscratch and load Cpuvar
                sd a0, {s0_scratch_offset}(tp)
                ld a0, {context_offset}(tp) // Load CpuVar.arch.context

                sd ra, {ra_offset}(a0)
                sd sp, {sp_offset}(a0)
                sd gp, {gp_offset}(a0)
                sd t0, {t0_offset}(a0)
                sd t1, {t1_offset}(a0)
                sd t2, {t2_offset}(a0)
                sd s0, {s0_offset}(a0)
                sd s1, {s1_offset}(a0)
                sd a1, {a1_offset}(a0)
                sd a2, {a2_offset}(a0)
                sd a3, {a3_offset}(a0)
                sd a4, {a4_offset}(a0)
                sd a5, {a5_offset}(a0)
                sd a6, {a6_offset}(a0)
                sd a7, {a7_offset}(a0)
                sd s2, {s2_offset}(a0)
                sd s3, {s3_offset}(a0)
                sd s4, {s4_offset}(a0)
                sd s5, {s5_offset}(a0)
                sd s6, {s6_offset}(a0)
                sd s7, {s7_offset}(a0)
                sd s8, {s8_offset}(a0)
                sd s9, {s9_offset}(a0)
                sd s10, {s10_offset}(a0)
                sd s11, {s11_offset}(a0)
                sd t3, {t3_offset}(a0)
                sd t4, {t4_offset}(a0)
                sd t5, {t5_offset}(a0)
                sd t6, {t6_offset}(a0)

                csrr a1, sepc
                sd a1, {sepc_offset}(a0)
                csrr a1, sstatus
                sd a1, {sstatus_offset}(a0)

                csrrw a1, sscratch, tp
                sd a1, {tp_offset}(a0)

                ld a1, {s0_scratch_offset}(tp)
                sd a1, {a0_offset}(a0)

                j {interrupt_handler}
            "#,
            context_offset = const offset_of!(crate::arch::CpuVar, context),
            s0_scratch_offset = const offset_of!(crate::arch::CpuVar, s0_scratch),
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
            interrupt_handler = sym interrupt_handler,
        )
    }
}

pub fn return_to_user(thread: *mut super::Thread, sysret: Option<isize>) -> ! {
    let context: *mut Context = unsafe { &mut (*thread).context as *mut _ };
    if let Some(value) = sysret {
        unsafe {
            (*context).a0 = value as usize;
        }
    }

    unsafe {
        asm!(r#"
            sd a0, {context_offset}(tp) // Update CpuVar.arch.context

            ld a1, {sepc_offset}(a0)
            csrw sepc, a1
            ld a1, {sstatus_offset}(a0)

            // Go back to the kernel mode. We don't yet support user mode.
            //
            // FIXME: Avoid updating sstatus here, and instead update it properly in
            //        kernel entry points.
            or a1, a1, 1 << 8

            csrw sstatus, a1

            // Restore general-purpose registers except tp.
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
