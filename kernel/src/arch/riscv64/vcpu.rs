use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use starina::error::ErrorCode;

use super::get_cpuvar;
use crate::arch::riscv64::csr::StvecMode;
use crate::arch::riscv64::csr::write_stvec;
use crate::arch::set_cpuvar;
use crate::cpuvar::CpuVar;
use crate::hvspace::HvSpace;
use crate::thread::switch_thread;

#[repr(C)]
#[derive(Debug, Default)]
pub struct VCpuState {
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

struct Context {
    state: VCpuState,
    cpuvar_ptr: u64,
}

pub struct VCpu {
    context: Context,
    hgatp: u64,
}

impl VCpu {
    pub fn new(hvspace: &HvSpace) -> Result<VCpu, ErrorCode> {
        let context = Context {
            state: VCpuState {
                ..Default::default()
            },
            cpuvar_ptr: 0,
        };

        Ok(VCpu {
            hgatp: hvspace.arch().hgatp(),
            context,
        })
    }
}

pub fn vcpu_entry(vcpu_ptr: *mut VCpu) -> ! {
    let cpuvar = get_cpuvar() as *const CpuVar;
    unsafe {
        (*vcpu_ptr).context.cpuvar_ptr = cpuvar as u64;

        asm!("csrw sscratch, {}", in(reg) &raw const (*vcpu_ptr).context);
        write_stvec(vcpu_trap_entry as *const () as usize, StvecMode::Direct);

        asm!(
            "csrw hgatp, {0}",
            in(reg) (*vcpu_ptr).hgatp,
            options(nostack),
        );

        // Prepare CSRs to go back to VS mode.
        let mut hstatus: u64;
        asm!("csrr {0}, hstatus", out(reg) hstatus);
        // SPV
        hstatus |= 1 << 7;
        // SPVP
        hstatus |= 1 << 8;
        asm!("csrw hstatus, {0}", in(reg) hstatus);

        let sepc: u64 = 0x8000c000;
        asm!("csrw sepc, {0}", in(reg) sepc);

        // Set the SPP bit to 0 to enter S-mode.
        let mut sstatus: u64;
        asm!("csrr {0}, sstatus", out(reg) sstatus);
        sstatus |= 1 << 8;
        asm!("csrw sstatus, {0}", in(reg) sstatus);

        asm!("sret");
    }

    unreachable!();
}

extern "C" fn vcpu_trap_handler(context: *mut Context) -> ! {
    trace!("exited from vCPU");

    unsafe {
        set_cpuvar((*context).cpuvar_ptr as *const CpuVar);
    }
    // TODO: restore CPU var (tp)
    switch_thread();
}

#[unsafe(naked)]
pub extern "C" fn vcpu_trap_entry() -> ! {
    unsafe {
        naked_asm!(
            // a0: *mut Context
            "csrrw a0, sscratch, a0",

            "sd ra, {ra_offset}(a0)",
            "sd sp, {sp_offset}(a0)",
            "sd gp, {gp_offset}(a0)",
            "sd tp, {tp_offset}(a0)",
            "sd t0, {t0_offset}(a0)",
            "sd t1, {t1_offset}(a0)",
            "sd t2, {t2_offset}(a0)",
            "sd s0, {s0_offset}(a0)",
            "sd s1, {s1_offset}(a0)",
            "sd a1, {a1_offset}(a0)",
            "sd a2, {a2_offset}(a0)",
            "sd a3, {a3_offset}(a0)",
            "sd a4, {a4_offset}(a0)",
            "sd a5, {a5_offset}(a0)",
            "sd a6, {a6_offset}(a0)",
            "sd a7, {a7_offset}(a0)",
            "sd s2, {s2_offset}(a0)",
            "sd s3, {s3_offset}(a0)",
            "sd s4, {s4_offset}(a0)",
            "sd s5, {s5_offset}(a0)",
            "sd s6, {s6_offset}(a0)",
            "sd s7, {s7_offset}(a0)",
            "sd s8, {s8_offset}(a0)",
            "sd s9, {s9_offset}(a0)",
            "sd s10, {s10_offset}(a0)",
            "sd s11, {s11_offset}(a0)",
            "sd t3, {t3_offset}(a0)",
            "sd t4, {t4_offset}(a0)",
            "sd t5, {t5_offset}(a0)",
            "sd t6, {t6_offset}(a0)",

            "csrr a1, sscratch",
            "sd a1, {a0_offset}(a0)",

            "j {vcpu_trap_handler}",

            vcpu_trap_handler = sym vcpu_trap_handler,
            ra_offset = const offset_of!(Context, state.ra),
            sp_offset = const offset_of!(Context, state.sp),
            gp_offset = const offset_of!(Context, state.gp),
            tp_offset = const offset_of!(Context, state.tp),
            t0_offset = const offset_of!(Context, state.t0),
            t1_offset = const offset_of!(Context, state.t1),
            t2_offset = const offset_of!(Context, state.t2),
            s0_offset = const offset_of!(Context, state.s0),
            s1_offset = const offset_of!(Context, state.s1),
            a0_offset = const offset_of!(Context, state.a0),
            a1_offset = const offset_of!(Context, state.a1),
            a2_offset = const offset_of!(Context, state.a2),
            a3_offset = const offset_of!(Context, state.a3),
            a4_offset = const offset_of!(Context, state.a4),
            a5_offset = const offset_of!(Context, state.a5),
            a6_offset = const offset_of!(Context, state.a6),
            a7_offset = const offset_of!(Context, state.a7),
            s2_offset = const offset_of!(Context, state.s2),
            s3_offset = const offset_of!(Context, state.s3),
            s4_offset = const offset_of!(Context, state.s4),
            s5_offset = const offset_of!(Context, state.s5),
            s6_offset = const offset_of!(Context, state.s6),
            s7_offset = const offset_of!(Context, state.s7),
            s8_offset = const offset_of!(Context, state.s8),
            s9_offset = const offset_of!(Context, state.s9),
            s10_offset = const offset_of!(Context, state.s10),
            s11_offset = const offset_of!(Context, state.s11),
            t3_offset = const offset_of!(Context, state.t3),
            t4_offset = const offset_of!(Context, state.t4),
            t5_offset = const offset_of!(Context, state.t5),
            t6_offset = const offset_of!(Context, state.t6),
        );
    };
}
