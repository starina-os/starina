use alloc::vec::Vec;
use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::vcpu::ExitInfo;
use starina::vcpu::ExitPageFault;
use starina::vcpu::ExitPageFaultKind;
use starina::vcpu::VCPU_EXIT_NONE;
use starina::vcpu::VCPU_EXIT_PAGE_FAULT;
use starina::vcpu::VCpuExitState;

use super::get_cpuvar;
use crate::arch::riscv64::csr::StvecMode;
use crate::arch::riscv64::csr::write_stvec;
use crate::arch::riscv64::entry::trap_entry;
use crate::arch::riscv64::riscv::SCAUSE_ECALL_FROM_VS;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_INST_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_LOAD_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_STORE_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_HOST_TIMER_INTR;
use crate::arch::riscv64::riscv::SCAUSE_VIRTUAL_INST;
use crate::arch::set_cpuvar;
use crate::cpuvar::CpuVar;
use crate::cpuvar::current_thread;
use crate::hvspace::HvSpace;
use crate::isolation::Isolation;
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
    vsip: u64,
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

impl Mutable {
    pub fn handle_sbi_call(&mut self, context: &Context) -> Result<isize, isize> {
        let fid = context.a6;
        let eid = context.a7;
        match (eid, fid) {
            (0x01, 0x00) => {
                let ch = context.a0 as u8;
                self.printer.putchar(ch);
                Ok(0)
            }
            (0x02, 0x00) => {
                // TODO: implement
                Ok(-1)
            }
            // Set timer
            (0x00, 0) => {
                panic!("SBI set_timer should not be called from VS/VU-mode");
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
        }
    }
}

fn handle_guest_page_fault(exit: &mut IsolationHeapMut, gpaddr: GPAddr, kind: ExitPageFaultKind) {
    info!(
        "handle_guest_page_fault: gpaddr={}, kind={:?}",
        gpaddr, kind
    );

    // FIXME: isolation
    exit.write(
        &Isolation::InKernel,
        0,
        VCpuExitState {
            reason: VCPU_EXIT_PAGE_FAULT,
            info: ExitInfo {
                page_fault: ExitPageFault {
                    gpaddr,
                    data: [0; 8],
                    kind,
                    width,
                },
            },
        },
    );
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
            htimedelta,
            vstimecmp: u64::MAX,
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

        let exit_state: VCpuExitState = match exit.read(&Isolation::InKernel, 0) {
            Ok(exit_state) => exit_state,
            Err(e) => {
                debug_warn!("failed to read exit state: {:?}", e);
                return Err(e);
            }
        };

        match exit_state.reason {
            VCPU_EXIT_PAGE_FAULT => {
                let page_fault = unsafe { &exit_state.info.page_fault };
                trace!(
                    "solving page fault: gpaddr={}, kind={:?}",
                    page_fault.gpaddr, page_fault.kind
                );

                // FIXME:
                let context = unsafe { &mut *((&self.context) as *const _ as *mut Context) };
            }
            VCPU_EXIT_NONE => {}
            _ => {
                trace!("unknown exit reason: {}", exit_state.reason);
                return Err(ErrorCode::InvalidState);
            }
        }

        mutable.exit = Some(exit);
        Ok(())
    }
}

pub fn vcpu_entry(vcpu: *mut VCpu) -> ! {
    unsafe {
        let cpuvar = get_cpuvar() as *const CpuVar;
        let context = &mut (*vcpu).context;
        context.cpuvar_ptr = cpuvar as u64;

        write_stvec(vcpu_trap_entry as *const () as usize, StvecMode::Direct);

        let hvip = context.hvip;
        context.hvip = 0;

        // Fill H extension CSRs and virtual CSRs.
        asm!(
            "csrw hgatp, {hgatp}",
            "hfence.gvma",

            "csrw hstatus, {hstatus}",
            "csrw hip, {hip}",
            "csrw hie, {hie}",
            "csrw hvip, {hvip}",
            "csrw htimedelta, {htimedelta}",

            "csrw sscratch, {sscratch}",
            "csrw sstatus, {sstatus}",
            "csrw sepc, {sepc}",

            "csrw vsstatus, {vsstatus}",
            "csrw vsie, {vsie}",
            "csrw vsip, {vsip}",
            "csrw vstvec, {vstvec}",
            "csrw vsscratch, {vsscratch}",
            "csrw vsatp, {vsatp}",
            "csrw vscause, {vscause}",
            "csrw vstval, {vstval}",
            "csrw vstimecmp, {vstimecmp}",

            hgatp = in(reg) context.hgatp,
            hstatus = in(reg) context.hstatus,
            hie = in(reg) context.hie,
            hip = in(reg) context.hip,
            hvip = in(reg) hvip,
            htimedelta = in(reg) context.htimedelta,
            sscratch = in(reg) vcpu as *const _ as u64,
            sstatus = in(reg) context.sstatus,
            sepc = in(reg) context.sepc,
            vsstatus = in(reg) context.vsstatus,
            vsie = in(reg) context.vsie,
            vsip = in(reg) context.vsip,
            vstvec = in(reg) context.vstvec,
            vsscratch = in(reg) context.vsscratch,
            vsatp = in(reg) context.vsatp,
            vscause = in(reg) context.vscause,
            vstval = in(reg) context.vstval,
            vstimecmp = in(reg) context.vstimecmp,
        );

        // Restore general-purpose registers and enter VS/VU-mode.
        asm!(
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

            // Restore a0 at the end of this switch - it contains the CpuVar pointer!
            "ld a0, {a0_offset}(a0)",
            "sret",
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

macro_rules! read_csr {
    ($csr:expr) => {{
        let mut value: u64;
        unsafe {
            asm!(concat!("csrr {}, ", $csr), out(reg) value);
        }
        value
    }};
}

extern "C" fn vcpu_trap_handler(vcpu: *mut VCpu) -> ! {
    let context = unsafe { &mut (*vcpu).context };
    debug_assert_eq!(context.magic, CONTEXT_MAGIC);

    context.vsstatus = read_csr!("vsstatus");
    context.vsepc = read_csr!("vsepc");
    context.vscause = read_csr!("vscause");
    context.vstval = read_csr!("vstval");
    context.vsie = read_csr!("vsie");
    context.vsip = read_csr!("vsip");
    context.vstvec = read_csr!("vstvec");
    context.vsscratch = read_csr!("vsscratch");
    context.vsatp = read_csr!("vsatp");
    context.vstimecmp = read_csr!("vstimecmp");
    context.hstatus = read_csr!("hstatus");
    context.hie = read_csr!("hie");
    context.hip = read_csr!("hip");
    context.sstatus = read_csr!("sstatus");
    context.sepc = read_csr!("sepc");

    if context.vstimecmp < u64::MAX {
        unsafe {
            // info!("vcpu_trap_handler: vstimecmp={:x}", context.vstimecmp);
            let mut now: u64;
            asm!("csrr {}, time", out(reg) now);
            let htime = now.wrapping_sub(context.htimedelta);
            asm!("csrw stimecmp, {}", in(reg) htime);
        }
    }

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

    // Check if it's from VS/VU-mode.
    assert!(context.hstatus & (1 << 7/* SPV */) != 0, "SPV is not set");

    let mut mutable = unsafe { (*vcpu).mutable.lock() };
    match scause {
        SCAUSE_HOST_TIMER_INTR => {
            let mut now: u64;
            unsafe {
                asm!("csrr {}, time", out(reg) now);
                asm!("csrw stimecmp, {}", in(reg) now + 0x100);
            }
        }
        SCAUSE_ECALL_FROM_VS => {
            let (error, value) = match mutable.handle_sbi_call(&context) {
                Ok(value) => (0, value),
                Err(error) => (error, 0),
            };

            context.sepc += 4; // size of ecall
            context.a0 = error as u64;
            context.a1 = value as u64;
        }
        SCAUSE_VIRTUAL_INST => {
            assert!(
                stval == 0x10500073,
                "Only WFI is expected in virtual instruction trap"
            );

            // info!(
            //     "vcpu_trap_handler: virtual instruction, sepc={:x}",
            //     context.sepc
            // );
            context.sepc += 4; // size of virtual instruction

            if context.hvip == 0 {
                // info!("no pending interrupt, going to idle");
                let current = current_thread();
                context.hvip = 1 << 6; // FIXME: We'll
                current.idle_vcpu();
            }
        }
        _ => {
            let exit = mutable.exit.as_mut().unwrap();
            match scause {
                SCAUSE_GUEST_INST_PAGE_FAULT => {
                    let gpaddr = GPAddr::new(htval as usize);
                    handle_guest_page_fault(exit, gpaddr, ExitPageFaultKind::Execute);
                }
                SCAUSE_GUEST_LOAD_PAGE_FAULT => {
                    let gpaddr = GPAddr::new(htval as usize);
                    handle_guest_page_fault(exit, gpaddr, ExitPageFaultKind::Load);
                }
                SCAUSE_GUEST_STORE_PAGE_FAULT => {
                    let gpaddr = GPAddr::new(htval as usize);
                    handle_guest_page_fault(exit, gpaddr, ExitPageFaultKind::Store);
                }
                _ => {
                    panic!(
                        "VM exit: {} (sepc={:x}, htval={:x}, stval={:x})",
                        scause_str,
                        unsafe { context.sepc },
                        htval,
                        stval
                    );
                }
            };

            current_thread().exit_vcpu();
        }
    }

    drop(mutable);
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
            cpuvar_ptr_offset = const offset_of!(VCpu, context.cpuvar_ptr),
            kernel_sp_offset = const offset_of!(CpuVar, arch.kernel_sp),
        );
    };
}

pub fn init() {
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
    let hcounteren: u64 = 0xffff_ffff;

    let mut henvcfg: u64 = 0;
    henvcfg |= 1 << 63; // STCE: STimecmp Enable

    unsafe {
        asm!("csrw hcounteren, {}", in(reg) hcounteren);
        asm!("csrw hedeleg, {}", in(reg) hedeleg);
        asm!("csrw hideleg, {}", in(reg) hideleg);
        asm!("csrw henvcfg, {}", in(reg) henvcfg);
    }
}
