use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use starina::error::ErrorCode;

use super::get_cpuvar;
use crate::arch::riscv64::csr::StvecMode;
use crate::arch::riscv64::csr::write_stvec;
use crate::arch::riscv64::entry::trap_entry;
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
    pub vsstatus: u64,
    pub vsepc: u64,
    pub vscause: u64,
    pub vstval: u64,
    pub vsie: u64,
    pub vstvec: u64,
    pub vsscratch: u64,
    pub vsatp: u64,
}

struct Context {
    state: VCpuState,
    cpuvar_ptr: u64,
    hvip: u64,
    hstatus: u64,
    sstatus: u64,
    sepc: u64,
}

pub struct VCpu {
    context: Context,
    hgatp: u64,
}

impl VCpu {
    pub fn new(hvspace: &HvSpace, entry: usize) -> Result<VCpu, ErrorCode> {
        let hstatus = 1 << 7; // SPV
        let sstatus = 1 << 8; // SPP

        let context = Context {
            state: VCpuState {
                ..Default::default()
            },
            cpuvar_ptr: 0,
            hvip: 0,
            sstatus: 0,
            sepc: entry as u64,
            hstatus,
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
        let context = &mut (*vcpu_ptr).context;
        context.cpuvar_ptr = cpuvar as u64;

        write_stvec(vcpu_trap_entry as *const () as usize, StvecMode::Direct);

        // Restore VS-mode CSRs
        asm!(
            "csrw sscratch, {sscratch}",
            "csrw sstatus, {sstatus}",
            "csrw sepc, {sepc}",
            "csrw vsstatus, {vsstatus}",
            "csrw vsie, {vsie}",
            "csrw vstvec, {vstvec}",
            "csrw vsscratch, {vsscratch}",
            "csrw vsatp, {vsatp}",
            "csrw vscause, {vscause}",
            "csrw vstval, {vstval}",
            "csrw hgatp, {hgatp}",
            "csrw hstatus, {hstatus}",
            "csrw hvip, {hvip}",
            "sret",
            sscratch = in(reg) &raw const context,
            sstatus = in(reg) context.sstatus,
            sepc = in(reg) context.sepc,
            vsstatus = in(reg) context.state.vsstatus,
            vsie = in(reg) context.state.vsie,
            vstvec = in(reg) context.state.vstvec,
            vsscratch = in(reg) context.state.vsscratch,
            vsatp = in(reg) context.state.vsatp,
            vscause = in(reg) context.state.vscause,
            vstval = in(reg) context.state.vstval,
            hgatp = in(reg) (*vcpu_ptr).hgatp,
            hstatus = in(reg) context.hstatus,
            hvip = in(reg) context.hvip,
            options(nostack),
        );
    }

    unreachable!();
}

extern "C" fn vcpu_trap_handler(context: *mut Context) -> ! {
    trace!("exited from vCPU");

    unsafe {
        debug_assert!((*context).cpuvar_ptr != 0);

        set_cpuvar((*context).cpuvar_ptr as *const CpuVar);
        write_stvec(trap_entry as *const () as usize, StvecMode::Direct);
    }

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

            "csrr t0, vsstatus",
            "sd t0, {vsstatus_offset}(a0)",
            "csrr t0, vsepc",
            "sd t0, {vsepc_offset}(a0)",
            "csrr t0, vscause",
            "sd t0, {vscause_offset}(a0)",
            "csrr t0, vstval",
            "sd t0, {vstval_offset}(a0)",
            "csrr t0, vsie",
            "sd t0, {vsie_offset}(a0)",
            "csrr t0, vstvec",
            "sd t0, {vstvec_offset}(a0)",
            "csrr t0, vsscratch",
            "sd t0, {vsscratch_offset}(a0)",
            "csrr t0, vsatp",
            "sd t0, {vsatp_offset}(a0)",
            "csrr t0, hstatus",
            "sd t0, {hstatus_offset}(a0)",
            "csrr t0, hvip",
            "sd t0, {hvip_offset}(a0)",
            "csrr t0, sstatus",
            "sd t0, {sstatus_offset}(a0)",
            "csrr t0, sepc",
            "sd t0, {sepc_offset}(a0)",

            "csrr t0, sscratch",
            "sd t0, {a0_offset}(a0)",

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
            vsstatus_offset = const offset_of!(Context, state.vsstatus),
            vsepc_offset = const offset_of!(Context, state.vsepc),
            vscause_offset = const offset_of!(Context, state.vscause),
            vstval_offset = const offset_of!(Context, state.vstval),
            vsie_offset = const offset_of!(Context, state.vsie),
            vstvec_offset = const offset_of!(Context, state.vstvec),
            vsscratch_offset = const offset_of!(Context, state.vsscratch),
            vsatp_offset = const offset_of!(Context, state.vsatp),
            hstatus_offset = const offset_of!(Context, hstatus),
            hvip_offset = const offset_of!(Context, hvip),
            sstatus_offset = const offset_of!(Context, sstatus),
            sepc_offset = const offset_of!(Context, sepc),
        );
    };
}
