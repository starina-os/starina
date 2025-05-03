use alloc::vec::Vec;
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
use crate::cpuvar::current_thread;
use crate::hvspace::HvSpace;
use crate::isolation::IsolationHeapMut;
use crate::spinlock::SpinLock;
use crate::thread::switch_thread;

const CONTEXT_MAGIC: u64 = 0xc000ffee;

#[repr(C)]
#[derive(Debug, Default)]
struct Context {
    magic: u64,
    cpuvar_ptr: u64,
    hgatp: u64,
    hie: u64,
    hip: u64,
    hvip: u64,
    hedeleg: u64,
    hideleg: u64,
    hcounteren: u64,
    htimedelta: u64,
    hstatus: u64,
    sstatus: u64,
    sepc: u64,
    ra: u64,
    sp: u64,
    gp: u64,
    tp: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
    t0: u64,
    t1: u64,
    t2: u64,
    t3: u64,
    t4: u64,
    t5: u64,
    t6: u64,
    vsstatus: u64,
    vsepc: u64,
    vscause: u64,
    vstval: u64,
    vsie: u64,
    vstvec: u64,
    vsscratch: u64,
    vsatp: u64,
    vstimecmp: u64,
}

struct ConsolePrinter {
    buf: Vec<u8>,
}

impl ConsolePrinter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn putchar(&mut self, ch: u8) {
        match ch {
            b'\n' => {
                info!(
                    "vCPU: \x1b[1;35m{}\x1b[0m",
                    str::from_utf8(&self.buf).unwrap_or("(non-UTF-8 data)")
                );
                self.buf.clear();
            }
            b'\r' => {
                // Do nothing.
            }
            _ => self.buf.push(ch),
        }
    }
}

struct Mutable {
    exit: Option<IsolationHeapMut>,
    printer: ConsolePrinter,
}

pub struct VCpu {
    context: Context,
    mutable: SpinLock<Mutable>,
}

impl VCpu {
    pub fn new(
        hvspace: &HvSpace,
        entry: usize,
        arg0: usize,
        arg1: usize,
    ) -> Result<VCpu, ErrorCode> {
        let sepc = entry as u64;

        let mut hstatus = 0;
        hstatus |= 1 << 7; // SPV
        hstatus |= 1 << 21; // VTW

        let sstatus = 1 << 8; // SPP
        let hgatp = hvspace.arch().hgatp();

        let mut hedeleg = 0;
        hedeleg |= 1 << 0; // Instruction address misaligned
        hedeleg |= 1 << 1; // Instruction access fault
        hedeleg |= 1 << 2; // Illegal instruction
        hedeleg |= 1 << 3; // Breakpoint
        hedeleg |= 1 << 4; // Load address misaligned
        hedeleg |= 1 << 5; // Load access fault
        hedeleg |= 1 << 6; // Store/AMO address misaligned
        hedeleg |= 1 << 7; // Store/AMO access fault
        hedeleg |= 1 << 8; // Environment call from U-mode
        hedeleg |= 1 << 12; // Instruction page fault
        hedeleg |= 1 << 13; // Load page fault
        hedeleg |= 1 << 15; // Store/AMO page fault

        let mut hideleg = 0;
        hideleg |= 1 << 6; // Supervisor timer interrupt

        // Enable all counters.
        let mut hcounteren = 0xffff_ffff;

        let mut now: u64;
        unsafe {
            asm!("rdtime {}", out(reg) now);
        }
        // let htimedelta = (-(now as i64)) as u64;
        let htimedelta = 0;

        let context = Context {
            magic: CONTEXT_MAGIC,
            sstatus,
            sepc,
            hgatp,
            hstatus,
            hedeleg,
            hideleg,
            hcounteren,
            htimedelta,
            a0: arg0 as u64,
            a1: arg1 as u64,
            ..Default::default()
        };

        let mutable = Mutable {
            exit: None,
            printer: ConsolePrinter::new(),
        };

        Ok(VCpu {
            context,
            mutable: SpinLock::new(mutable),
        })
    }

    pub fn update(&self, exit: IsolationHeapMut) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.exit.is_some() {
            debug_warn!("vCPU already in use");
            return Err(ErrorCode::InUse);
        }

        mutable.exit = Some(exit);
        Ok(())
    }
}

pub fn vcpu_entry(vcpu: *mut VCpu) -> ! {
    let cpuvar = get_cpuvar() as *const CpuVar;
    unsafe {
        let context = &mut (*vcpu).context;
        context.cpuvar_ptr = cpuvar as u64;

        // info!("vcpu_entry: sepc={:x}", context.sepc);

        write_stvec(vcpu_trap_entry as *const () as usize, StvecMode::Direct);

        // Restore VS-mode CSRs
        asm!(
            "csrw hgatp, {hgatp}",
            "hfence.gvma",

            "csrw hstatus, {hstatus}",
            "csrw hip, {hip}",
            "csrw hie, {hie}",
            "csrw hvip, {hvip}",
            "csrw hedeleg, {hedeleg}",
            "csrw hideleg, {hideleg}",
            "csrw hcounteren, {hcounteren}",
            "csrw htimedelta, {htimedelta}",

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
            "csrw vstimecmp, {vstimecmp}",

            // Restore general-purpose registers
            "mv a0, {context}",
            "ld ra, {ra_offset}(a0)",
            "ld sp, {sp_offset}(a0)",
            "ld gp, {gp_offset}(a0)",
            "ld tp, {tp_offset}(a0)",
            "ld t0, {t0_offset}(a0)",
            "ld t1, {t1_offset}(a0)",
            "ld t2, {t2_offset}(a0)",
            "ld s0, {s0_offset}(a0)",
            "ld s1, {s1_offset}(a0)",
            "ld a1, {a1_offset}(a0)",
            "ld a2, {a2_offset}(a0)",
            "ld a3, {a3_offset}(a0)",
            "ld a4, {a4_offset}(a0)",
            "ld a5, {a5_offset}(a0)",
            "ld a6, {a6_offset}(a0)",
            "ld a7, {a7_offset}(a0)",
            "ld s2, {s2_offset}(a0)",
            "ld s3, {s3_offset}(a0)",
            "ld s4, {s4_offset}(a0)",
            "ld s5, {s5_offset}(a0)",
            "ld s6, {s6_offset}(a0)",
            "ld s7, {s7_offset}(a0)",
            "ld s8, {s8_offset}(a0)",
            "ld s9, {s9_offset}(a0)",
            "ld s10, {s10_offset}(a0)",
            "ld s11, {s11_offset}(a0)",
            "ld t3, {t3_offset}(a0)",
            "ld t4, {t4_offset}(a0)",
            "ld t5, {t5_offset}(a0)",
            "ld t6, {t6_offset}(a0)",

            "ld a0, {a0_offset}(a0)",

            "sret",
            hgatp = in(reg) context.hgatp,
            hstatus = in(reg) context.hstatus,
            hie = in(reg) context.hie,
            hip = in(reg) context.hip,
            hvip = in(reg) context.hvip,
            hedeleg = in(reg) context.hedeleg,
            hideleg = in(reg) context.hideleg,
            hcounteren = in(reg) context.hcounteren,
            htimedelta = in(reg) context.htimedelta,
            sscratch = in(reg) vcpu as *const _ as u64,
            sstatus = in(reg) context.sstatus,
            sepc = in(reg) context.sepc,
            vsstatus = in(reg) context.vsstatus,
            vsie = in(reg) context.vsie,
            vstvec = in(reg) context.vstvec,
            vsscratch = in(reg) context.vsscratch,
            vsatp = in(reg) context.vsatp,
            vscause = in(reg) context.vscause,
            vstval = in(reg) context.vstval,
            vstimecmp = in(reg) context.vstimecmp,
            context = in(reg) context as *const _,
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
            options(nostack),
        );
    }

    unreachable!();
}

extern "C" fn vcpu_trap_handler(vcpu: *mut VCpu) -> ! {
    let context = unsafe { &raw mut (*vcpu).context };

    let scause: u64;
    let stval: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) scause);
        asm!("csrr {}, stval", out(reg) stval);
    }

    let is_intr = scause & (1 << 63) != 0;
    let code = scause & !(1 << 63);
    let scause_str = match (is_intr, code) {
        (true, 0) => "user software interrupt",
        (true, 1) => "supervisor software interrupt",
        (true, 2) => "hypervisor software interrupt",
        (true, 3) => "machine software interrupt",
        (true, 4) => "user timer interrupt",
        (true, 5) => "supervisor timer interrupt",
        (true, 6) => "hypervisor timer interrupt",
        (true, 7) => "machine timer interrupt",
        (true, 8) => "user external interrupt",
        (true, 9) => "supervisor external interrupt",
        (true, 10) => "hypervisor external interrupt",
        (true, 11) => "machine external interrupt",
        (false, 0) => "instruction address misaligned",
        (false, 1) => "instruction access fault",
        (false, 2) => "illegal instruction",
        (false, 3) => "breakpoint",
        (false, 4) => "load address misaligned",
        (false, 5) => "load access fault",
        (false, 6) => "store/AMO address misaligned",
        (false, 7) => "store/AMO access fault",
        (false, 8) => "environment call from U-mode",
        (false, 9) => "environment call from S-mode",
        (false, 10) => "Environment call from VS-mode",
        (false, 11) => "environment call from M-mode",
        (false, 12) => "instruction page fault",
        (false, 13) => "load page fault",
        (false, 15) => "store/AMO page fault",
        (false, 20) => "instruction guest-page fault",
        (false, 21) => "load guest-page fault",
        (false, 22) => "virtual instruction",
        (false, 23) => "store/AMO guest-page fault",
        _ => "unknown",
    };

    let scause_code = scause & 0x7ff;

    let mut htval: u64;
    unsafe {
        asm!("csrr {}, htval", out(reg) htval);
    }

    unsafe {
        write_stvec(trap_entry as *const () as usize, StvecMode::Direct);
    }

    {
        let mut mutable = unsafe { (*vcpu).mutable.lock() };
        if scause_code == 10 && unsafe { (*context).a7 } == 1 {
            // info!("ecall: a0={:x}", unsafe { (*context).a0 });
            let ch = unsafe { (*context).a0 } as u8;
            mutable.printer.putchar(ch);
            unsafe {
                (*context).sepc += 4; // size of ecall
            }
        } else if scause == 10 {
            // a7 encodes the SBI extension ID (EID),
            // a6 encodes the SBI function ID (FID) for a given extension ID encoded in a7 for any SBI extension defined in or after SBI v0.2.
            // info!(
            //     "ecall: a0={:x}, eid={:x}, fid={:x}, sepc={:x}",
            //     unsafe { (*context).a0 },
            //     unsafe { (*context).a7 },
            //     unsafe { (*context).a6 },
            //     unsafe { (*context).sepc }
            // );

            let a0 = unsafe { (*context).a0 };
            let fid = unsafe { (*context).a6 };
            let eid = unsafe { (*context).a7 };
            let result = match (eid, fid) {
                // Set timer
                (0x00, 0) => {
                    // TODO: implement
                    info!("SBI: set_timer: a0={:x}", a0);
                    if a0 < 0xffff_ffff_ffff {}
                    Ok(0)
                }
                //  Get SBI specification version
                (0x10, 0) => {
                    //  version 0.1
                    Ok(0x01)
                }
                // Probe SBI extension
                (0x10, 3) => {
                    // 0 means the extension is not supported.
                    Ok(0)
                }
                // Get machine vendor ID
                (0x10, 4) => {
                    // "0 is always a legal value for this CSR" as per SBI spec.
                    Ok(0)
                }
                // Get machine architecture ID
                (0x10, 5) => {
                    // "0 is always a legal value for this CSR" as per SBI spec.
                    Ok(0)
                }
                // Get machine implementation ID
                (0x10, 6) => {
                    // "0 is always a legal value for this CSR" as per SBI spec.
                    Ok(0)
                }
                _ => {
                    panic!("SBI: unknown eid={:x}, fid={:x}", eid, fid);
                }
            };

            let (error, value) = match result {
                Ok(value) => (0, value),
                Err(error) => (error, 0),
            };

            unsafe {
                (*context).sepc += 4; // size of ecall
            }

            unsafe {
                (*context).a0 = error;
                (*context).a1 = value;
            }
            // virtual instruction
        } else if scause == 22 {
            trace!("virtual instruction: sepc={:x}", unsafe { (*context).sepc });

            info!("injecting timer interrupt");
            unsafe {
                // VSTIP: supervisor timer interrupt pending
                (*context).hvip |= 1 << 6;
            }

            // let htinst: u64;
            // let vsie: u64;
            // let hie: u64;
            // unsafe {
            //     asm!("csrr {}, htinst", out(reg) htinst);
            //     asm!("csrr {}, vsie", out(reg) vsie);
            //     asm!("csrr {}, hie", out(reg) hie);
            // }
            // panic!(
            //     "virtual instruction: sepc={:x}, vsie={:x}, hie={:x}",
            //     unsafe { (*context).sepc },
            //     vsie,
            //     hie
            // );
            // let mut hvip = unsafe { (*context).hvip };
            // // Set VSTIP
            // hvip |= 1 << 6;
            // // VSSIP
            // hvip |= 1 << 2;
            // unsafe {
            //     (*context).hvip = hvip;
            // }
        } else {
            panic!(
                "VM exit: {} (sepc={:x}, htval={:x}, stval={:x})",
                scause_str,
                unsafe { (*context).sepc },
                htval,
                stval
            );
        }

        // let exit = mutable.exit.take().unwrap();
        // drop(mutable);
        // current_thread().exit_vcpu();
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
            "csrr t0, vstimecmp",
            "sd t0, {vstimecmp_offset}(a0)",
            "csrr t0, hstatus",
            "sd t0, {hstatus_offset}(a0)",
            "csrr t0, hie",
            "sd t0, {hie_offset}(a0)",
            "csrr t0, hip",
            "sd t0, {hip_offset}(a0)",
            "csrr t0, hvip",
            "sd t0, {hvip_offset}(a0)",
            "csrr t0, hcounteren",
            "sd t0, {hcounteren_offset}(a0)",
            "csrr t0, sstatus",
            "sd t0, {sstatus_offset}(a0)",
            "csrr t0, sepc",
            "sd t0, {sepc_offset}(a0)",

            "csrr t0, sscratch",
            "sd t0, {a0_offset}(a0)",

            // Restore the CpuVar pointer.
            "ld tp, {cpuvar_ptr_offset}(a0)",
            "csrw sscratch, tp",

            "ld sp, {kernel_sp_offset}(tp)",
            "j {vcpu_trap_handler}",

            vcpu_trap_handler = sym vcpu_trap_handler,
            ra_offset = const offset_of!(VCpu, context.ra),
            sp_offset = const offset_of!(VCpu, context.sp),
            gp_offset = const offset_of!(VCpu, context.gp),
            tp_offset = const offset_of!(VCpu, context.tp),
            t0_offset = const offset_of!(VCpu, context.t0),
            t1_offset = const offset_of!(VCpu, context.t1),
            t2_offset = const offset_of!(VCpu, context.t2),
            s0_offset = const offset_of!(VCpu, context.s0),
            s1_offset = const offset_of!(VCpu, context.s1),
            a0_offset = const offset_of!(VCpu, context.a0),
            a1_offset = const offset_of!(VCpu, context.a1),
            a2_offset = const offset_of!(VCpu, context.a2),
            a3_offset = const offset_of!(VCpu, context.a3),
            a4_offset = const offset_of!(VCpu, context.a4),
            a5_offset = const offset_of!(VCpu, context.a5),
            a6_offset = const offset_of!(VCpu, context.a6),
            a7_offset = const offset_of!(VCpu, context.a7),
            s2_offset = const offset_of!(VCpu, context.s2),
            s3_offset = const offset_of!(VCpu, context.s3),
            s4_offset = const offset_of!(VCpu, context.s4),
            s5_offset = const offset_of!(VCpu, context.s5),
            s6_offset = const offset_of!(VCpu, context.s6),
            s7_offset = const offset_of!(VCpu, context.s7),
            s8_offset = const offset_of!(VCpu, context.s8),
            s9_offset = const offset_of!(VCpu, context.s9),
            s10_offset = const offset_of!(VCpu, context.s10),
            s11_offset = const offset_of!(VCpu, context.s11),
            t3_offset = const offset_of!(VCpu, context.t3),
            t4_offset = const offset_of!(VCpu, context.t4),
            t5_offset = const offset_of!(VCpu, context.t5),
            t6_offset = const offset_of!(VCpu, context.t6),
            vsstatus_offset = const offset_of!(VCpu, context.vsstatus),
            vsepc_offset = const offset_of!(VCpu, context.vsepc),
            vscause_offset = const offset_of!(VCpu, context.vscause),
            vstval_offset = const offset_of!(VCpu, context.vstval),
            vsie_offset = const offset_of!(VCpu, context.vsie),
            vstvec_offset = const offset_of!(VCpu, context.vstvec),
            vsscratch_offset = const offset_of!(VCpu, context.vsscratch),
            vsatp_offset = const offset_of!(VCpu, context.vsatp),
            vstimecmp_offset = const offset_of!(VCpu, context.vstimecmp),
            hstatus_offset = const offset_of!(VCpu, context.hstatus),
            hie_offset = const offset_of!(VCpu, context.hie),
            hip_offset = const offset_of!(VCpu, context.hip),
            hvip_offset = const offset_of!(VCpu, context.hvip),
            hcounteren_offset = const offset_of!(VCpu, context.hcounteren),
            sstatus_offset = const offset_of!(VCpu, context.sstatus),
            sepc_offset = const offset_of!(VCpu, context.sepc),
            cpuvar_ptr_offset = const offset_of!(VCpu, context.cpuvar_ptr),
            kernel_sp_offset = const offset_of!(CpuVar, arch.kernel_sp),
        );
    };
}
