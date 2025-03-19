use core::arch::naked_asm;
use core::mem::offset_of;

use super::cpuvar::CpuVar;
use super::interrupt::interrupt_handler;
use super::thread::Context;

/// The entry point for traps: exceptions, interrupts, and system calls.
#[naked]
pub unsafe extern "C" fn switch_to_kernel() -> ! {
    unsafe {
        naked_asm!(
            r#"
                csrrw tp, sscratch, tp      // Save tp to sscratch and load Cpuvar
                sd a0, {a0_scratch_offset}(tp)
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

                ld a1, {a0_scratch_offset}(tp)
                sd a1, {a0_offset}(a0)

                ld sp, {kernel_sp_offset}(tp)
                j {interrupt_handler}
            "#,
            context_offset = const offset_of!(CpuVar, context),
            a0_scratch_offset = const offset_of!(CpuVar, a0_scratch),
            kernel_sp_offset = const offset_of!(CpuVar, kernel_sp),
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
