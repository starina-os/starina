use alloc::vec::Vec;
use core::arch::asm;
use core::arch::naked_asm;
use core::mem::offset_of;

use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::vcpu::ExitInfo;
use starina::vcpu::ExitPageFault;
use starina::vcpu::ExitPageFaultKind;
use starina::vcpu::LoadInst;
use starina::vcpu::VCPU_EXIT_IDLE;
use starina::vcpu::VCPU_EXIT_NONE;
use starina::vcpu::VCPU_EXIT_PAGE_FAULT;
use starina::vcpu::VCPU_EXIT_REBOOT;
use starina::vcpu::VCpuRunState;

use super::get_cpuvar;
use crate::arch::riscv64::csr::StvecMode;
use crate::arch::riscv64::csr::write_stvec;
use crate::arch::riscv64::entry::trap_entry;
use crate::arch::riscv64::riscv::OP_LOAD_FUNCT3_LB;
use crate::arch::riscv64::riscv::OP_LOAD_FUNCT3_LD;
use crate::arch::riscv64::riscv::OP_LOAD_FUNCT3_LH;
use crate::arch::riscv64::riscv::OP_LOAD_FUNCT3_LW;
use crate::arch::riscv64::riscv::OP_STORE_FUNCT3_SB;
use crate::arch::riscv64::riscv::OP_STORE_FUNCT3_SD;
use crate::arch::riscv64::riscv::OP_STORE_FUNCT3_SH;
use crate::arch::riscv64::riscv::OP_STORE_FUNCT3_SW;
use crate::arch::riscv64::riscv::SCAUSE_ECALL_FROM_VS;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_INST_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_LOAD_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_GUEST_STORE_PAGE_FAULT;
use crate::arch::riscv64::riscv::SCAUSE_HOST_TIMER_INTR;
use crate::arch::riscv64::riscv::SCAUSE_SV_EXT_INTR;
use crate::arch::riscv64::riscv::SCAUSE_VIRTUAL_INST;
use crate::cpuvar::CpuVar;
use crate::cpuvar::current_thread;
use crate::hvspace::HvSpace;
use crate::isolation::Isolation;
use crate::isolation::IsolationSliceMut;
use crate::spinlock::SpinLock;
use crate::thread::switch_thread;

const CONTEXT_MAGIC: u64 = 0xc000ffee;

/// VS-level external interrupt (e.g. virtio interrupts).
const HVIP_VSEIP: u64 = 1 << 10;

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

impl Context {
    pub fn get_reg(&self, rs: u8) -> u64 {
        match rs {
            0 => 0,
            1 => self.ra,
            2 => self.sp,
            3 => self.gp,
            4 => self.tp,
            5 => self.t0,
            6 => self.t1,
            7 => self.t2,
            8 => self.s0,
            9 => self.s1,
            10 => self.a0,
            11 => self.a1,
            12 => self.a2,
            13 => self.a3,
            14 => self.a4,
            15 => self.a5,
            16 => self.a6,
            17 => self.a7,
            18 => self.s2,
            19 => self.s3,
            20 => self.s4,
            21 => self.s5,
            22 => self.s6,
            23 => self.s7,
            24 => self.s8,
            25 => self.s9,
            26 => self.s10,
            27 => self.s11,
            28 => self.t3,
            29 => self.t4,
            30 => self.t5,
            31 => self.t6,
            _ => {
                panic!("unknown rs: {}", rs);
            }
        }
    }

    pub fn set_reg(&mut self, rd: u8, value: u64) {
        match rd {
            0 => {} // Do nothing.
            1 => self.ra = value,
            2 => self.sp = value,
            3 => self.gp = value,
            4 => self.tp = value,
            5 => self.t0 = value,
            6 => self.t1 = value,
            7 => self.t2 = value,
            8 => self.s0 = value,
            9 => self.s1 = value,
            10 => self.a0 = value,
            11 => self.a1 = value,
            12 => self.a2 = value,
            13 => self.a3 = value,
            14 => self.a4 = value,
            15 => self.a5 = value,
            16 => self.a6 = value,
            17 => self.a7 = value,
            18 => self.s2 = value,
            19 => self.s3 = value,
            20 => self.s4 = value,
            21 => self.s5 = value,
            22 => self.s6 = value,
            23 => self.s7 = value,
            24 => self.s8 = value,
            25 => self.s9 = value,
            26 => self.s10 = value,
            27 => self.s11 = value,
            28 => self.t3 = value,
            29 => self.t4 = value,
            30 => self.t5 = value,
            31 => self.t6 = value,
            _ => panic!("unknown rd: {}", rd),
        }
    }
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

struct PlicEmu {
    pending_irqs: u32,
}

impl PlicEmu {
    pub fn new() -> Self {
        Self { pending_irqs: 0 }
    }

    pub fn update(&mut self, irqs: u32) {
        self.pending_irqs |= irqs;
    }

    pub fn is_pending(&self) -> bool {
        self.pending_irqs != 0
    }

    pub fn peek_pending_irq(&self) -> Option<u8> {
        if self.pending_irqs == 0 {
            return None;
        }

        let irq = self.pending_irqs.trailing_zeros();
        Some(irq as u8)
    }

    pub fn acknowledge_irq(&mut self, irq: u8) {
        self.pending_irqs &= !(1 << irq);
    }

    pub fn mmio_read(&self, offset: u64, width: u8) -> u64 {
        if width != 4 {
            panic!("plic-emu: mmio_read: invalid width: {}", width);
        }

        match offset {
            0x2000..0x200000 => {
                // Enable bits.
                0
            }
            0x200000..0x4000000 => {
                //
                match offset & 0xfff {
                    0x000 => {
                        // Priority threshold.
                        0
                    }
                    0x004 => {
                        // Claim.
                        self.peek_pending_irq().unwrap_or(0).into()
                    }
                    _ => {
                        panic!("plic-emu: mmio_read: unknown offset: {:x}", offset);
                    }
                }
            }
            _ => {
                debug_warn!("plic-emu: mmio_read: unknown offset: {:x}", offset);
                0
            }
        }
    }

    pub fn mmio_write(&mut self, offset: u64, value: u64, width: u8) {
        if width != 4 {
            panic!("plic-emu: mmio_write: invalid width: {}", width);
        }

        match offset {
            0x200000..0x4000000 => {
                //
                match offset & 0xfff {
                    0x000 => {
                        // Priority threshold.
                    }
                    0x004 => {
                        // Claim.
                        self.acknowledge_irq(value as u8);
                    }
                    _ => {
                        panic!("plic-emu: mmio_read: unknown offset: {:x}", offset);
                    }
                }
            }
            _ => {
                debug_warn!("plic-emu: mmio_write: unknown offset: {:x}", offset);
            }
        }
    }
}
struct Mutable {
    run_state_slice: Option<IsolationSliceMut>,
    printer: ConsolePrinter,
    plic: PlicEmu,
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
                Err(-1)
            }
            // Set timer
            (0x00, 0) => {
                panic!("SBI set_timer should not be called from VS/VU-mode");
            }
            //  Get SBI specification version
            (0x10, 0) => {
                //  version 0.3
                Ok(0x03)
            }
            // Get SBI implementation ID
            (0x10, 1) => {
                // A placeholder value.
                Ok(0x00)
            }
            // Get SBI implementation version
            (0x10, 2) => {
                // A placeholder value.
                Ok(0x00)
            }
            // Probe SBI extension
            (0x10, 3) => {
                if context.a0 == 0x53525354
                /* SRST */
                {
                    // Supported.
                    Ok(1)
                } else {
                    // 0 means the extension is not supported.
                    Ok(0)
                }
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
            // System reset
            (0x53525354, 0x00) => {
                self.trigger_vm_exit(VCPU_EXIT_REBOOT, ExitInfo::empty());
                Ok(0)
            }
            _ => {
                panic!("SBI: unknown eid={:x}, fid={:x}", eid, fid);
            }
        }
    }

    fn trigger_vm_exit(&mut self, exit_reason: u8, exit_info: ExitInfo) {
        let current_thread = current_thread();
        let isolation = current_thread.process().isolation();
        current_thread.exit_vcpu();

        // FIXME: Handle error.
        let exit = self.run_state_slice.take().expect("tried to VM-exit twice");
        let _ = exit.write(
            isolation,
            offset_of!(VCpuRunState, exit_reason),
            exit_reason,
        );
        let _ = exit.write(isolation, offset_of!(VCpuRunState, exit_info), exit_info);
    }
}

fn htinst_load_inst(htinst: u64) -> (LoadInst, u8, u8) {
    if htinst & 1 != 1 {
        panic!("invalid load instruction: {:x}", htinst);
    }

    let is_compressed = htinst & (1 << 1) == 0;
    let inst_len = if is_compressed { 2 } else { 4 };
    let inst = htinst | (1 << 1);

    let opcode = (inst & 0b111_1111) as u8;
    let rd = ((inst >> 7) & 0b1_1111) as u8;
    let funct3 = ((inst >> 12) & 0b111) as u8;
    let addr_offset = ((inst >> 15) & 0b1_1111) as u8;

    assert!(addr_offset == 0); // TODO: Handle misaligned load.

    if opcode != 3 {
        panic!("unsupported opcode for load: {:x}", opcode);
    }

    match funct3 {
        OP_LOAD_FUNCT3_LB => (LoadInst { rd }, 1, inst_len),
        OP_LOAD_FUNCT3_LH => (LoadInst { rd }, 2, inst_len),
        OP_LOAD_FUNCT3_LW => (LoadInst { rd }, 4, inst_len),
        OP_LOAD_FUNCT3_LD => (LoadInst { rd }, 8, inst_len),
        _ => {
            panic!("unsupported funct3 for load: {:x}", funct3);
        }
    }
}

fn htinst_store_inst(context: &Context, htinst: u64) -> (u8, [u8; 8], u8) {
    if htinst & 1 != 1 {
        panic!("invalid store instruction: {:x}", htinst);
    }

    let is_compressed = htinst & (1 << 1) == 0;
    let inst_len = if is_compressed { 2 } else { 4 };
    let inst = htinst | (1 << 1);

    let opcode = (inst & 0b111_1111) as u8;
    let funct3 = ((inst >> 12) & 0b111) as u8;
    let addr_offset = ((inst >> 15) & 0b1_1111) as u8;
    let rs2 = ((inst >> 20) & 0b1_1111) as u8;

    assert!(addr_offset == 0); // TODO: Handle misaligned store.

    if opcode != 0b100011 {
        panic!("unsupported opcode for store: {:x}", opcode);
    }

    let width = match funct3 {
        OP_STORE_FUNCT3_SB => 1,
        OP_STORE_FUNCT3_SH => 2,
        OP_STORE_FUNCT3_SW => 4,
        OP_STORE_FUNCT3_SD => 8,
        _ => panic!("unsupported funct3 for store: {:x}", funct3),
    };

    let value = context.get_reg(rs2);
    let mut data = [0; 8];
    match width {
        1 => {
            data[0] = value as u8;
        }
        2 => {
            data[0..2].copy_from_slice(&value.to_le_bytes()[0..2]);
        }
        4 => {
            data[0..4].copy_from_slice(&value.to_le_bytes()[0..4]);
        }
        8 => {
            data.copy_from_slice(&value.to_le_bytes());
        }
        _ => {
            unreachable!();
        }
    }

    (width, data, inst_len)
}

const fn plic_mmio_size(num_cpus: u32) -> usize {
    0x200000 + (num_cpus as usize * 0x1000)
}

const PLIC_ADDR: GPAddr = GPAddr::new(0x0a00_0000);
const PLIC_SIZE: usize = plic_mmio_size(1); // FIXME:

fn handle_guest_page_fault(
    mutable: &mut Mutable,
    context: &mut Context,
    htinst: u64,
    gpaddr: GPAddr,
    kind: ExitPageFaultKind,
) {
    // info!(
    //     "handle_guest_page_fault: gpaddr={}, kind={:?}",
    //     gpaddr, kind
    // );

    let (load_inst, data, width, inst_len) = match kind {
        ExitPageFaultKind::Load | ExitPageFaultKind::Execute => {
            let (load_inst, width, inst_len) = htinst_load_inst(htinst);
            (load_inst, [0; 8], width, inst_len)
        }
        ExitPageFaultKind::Store => {
            let (width, data, inst_len) = htinst_store_inst(context, htinst);
            (LoadInst::default(), data, width, inst_len)
        }
        _ => {
            panic!("unknown exit page fault kind: {:?}", kind);
        }
    };

    let plic_end = PLIC_ADDR.checked_add(PLIC_SIZE).unwrap(); // FIXME:
    if PLIC_ADDR <= gpaddr && gpaddr < plic_end {
        let offset = (gpaddr.as_usize() - PLIC_ADDR.as_usize()) as u64;
        match kind {
            ExitPageFaultKind::Store => {
                let value = match width {
                    1 => data[0] as u64,
                    2 => u16::from_ne_bytes([data[0], data[1]]) as u64,
                    4 => u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as u64,
                    8 => u64::from_ne_bytes(data),
                    _ => {
                        panic!("unknown width: {}", width);
                    }
                };
                mutable.plic.mmio_write(offset, value, width);
            }
            _ => {
                let value = mutable.plic.mmio_read(offset, width);
                context.set_reg(load_inst.rd, value);
            }
        }

        context.sepc += inst_len as u64;
        return;
    }

    mutable.trigger_vm_exit(
        VCPU_EXIT_PAGE_FAULT,
        ExitInfo::page_fault(ExitPageFault {
            gpaddr,
            data,
            kind,
            width,
            load_inst,
            inst_len,
        }),
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
        hstatus |= 2u64 << 32; // VSXL (64-bit)
        hstatus |= 1 << 7; // SPV
        hstatus |= 1 << 21; // VTW
        hstatus |= 3 << 13; // FP

        let mut sstatus = 0;
        sstatus |= 1 << 8; // SPP
        sstatus &= !(0b11 << 13); // Clear FP
        sstatus |= 3 << 13; // FP

        let hgatp = hvspace.arch().hgatp();

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
            run_state_slice: None,
            printer: ConsolePrinter::new(),
            plic: PlicEmu::new(),
        };

        Ok(VCpu {
            context,
            mutable: SpinLock::new(mutable),
        })
    }

    pub fn apply_state(
        &self,
        isolation: &dyn Isolation,
        run_state_slice: IsolationSliceMut,
    ) -> Result<(), ErrorCode> {
        let mut mutable = self.mutable.lock();
        if mutable.run_state_slice.is_some() {
            debug_warn!("vCPU already in use");
            return Err(ErrorCode::InUse);
        }

        let run_state: VCpuRunState = match run_state_slice.read(isolation, 0) {
            Ok(run_state) => run_state,
            Err(e) => {
                debug_warn!("failed to read run state: {:?}", e);
                return Err(e);
            }
        };

        let irqs = run_state.irqs;
        if irqs != 0 {
            mutable.plic.update(irqs);
        }

        // FIXME:
        let context = {
            let ptr = (&self.context) as *const _ as usize;
            ptr as *mut Context
        };

        match run_state.exit_reason {
            VCPU_EXIT_PAGE_FAULT => {
                let page_fault = run_state.exit_info.as_page_fault();

                unsafe {
                    (*context).sepc += page_fault.inst_len as u64;
                }

                match page_fault.kind {
                    ExitPageFaultKind::Load | ExitPageFaultKind::Execute => unsafe {
                        let value = match page_fault.width {
                            1 => page_fault.data[0] as u64,
                            2 => {
                                u16::from_le_bytes([page_fault.data[0], page_fault.data[1]]) as u64
                            }
                            4 => {
                                u32::from_le_bytes([
                                    page_fault.data[0],
                                    page_fault.data[1],
                                    page_fault.data[2],
                                    page_fault.data[3],
                                ]) as u64
                            }
                            8 => {
                                u64::from_le_bytes([
                                    page_fault.data[0],
                                    page_fault.data[1],
                                    page_fault.data[2],
                                    page_fault.data[3],
                                    page_fault.data[4],
                                    page_fault.data[5],
                                    page_fault.data[6],
                                    page_fault.data[7],
                                ])
                            }
                            _ => panic!("unknown load width: {}", page_fault.width),
                        };

                        (*context).set_reg(page_fault.load_inst.rd, value);
                    },
                    ExitPageFaultKind::Store => {}
                    _ => {
                        panic!("unknown exit page fault kind: {:?}", run_state.exit_reason);
                    }
                }
            }
            VCPU_EXIT_IDLE | VCPU_EXIT_NONE => {}
            _ => {
                trace!("unknown exit reason: {}", run_state.exit_reason);
                return Err(ErrorCode::InvalidState);
            }
        }

        mutable.run_state_slice = Some(run_state_slice);
        Ok(())
    }
}

pub fn vcpu_entry(vcpu: *mut VCpu) -> ! {
    unsafe {
        let cpuvar = get_cpuvar() as *const CpuVar;
        let context = &mut (*vcpu).context;
        context.cpuvar_ptr = cpuvar as u64;

        write_stvec(vcpu_trap_entry as *const () as usize, StvecMode::Direct);

        let mut hvip = context.hvip;
        context.hvip = 0;

        let irq_pending = {
            let mutable = (*vcpu).mutable.lock();
            mutable.plic.is_pending()
        };

        if irq_pending {
            // info!("vCPU: injecting HVIP_VSEIP");
            hvip |= HVIP_VSEIP;
        }

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

fn htval_to_gpaddr(htval: u64, stval: u64) -> GPAddr {
    // "A guest physical address written to htval is shifted right by 2 bits"
    let upper = htval << 2;
    let lower = stval & 0b11;
    GPAddr::new((upper | lower) as usize)
}

fn scause_to_string(is_intr: bool, code: u64) -> &'static str {
    match (is_intr, code) {
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
        (false, 10) => "environment call from VS-mode",
        (false, 11) => "environment call from M-mode",
        (false, 12) => "instruction page fault",
        (false, 13) => "load page fault",
        (false, 15) => "store/AMO page fault",
        (false, 20) => "instruction guest-page fault",
        (false, 21) => "load guest-page fault",
        (false, 22) => "virtual instruction",
        (false, 23) => "store/AMO guest-page fault",
        _ => "unknown",
    }
}

fn save_virtual_csrs(context: &mut Context) {
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
}

fn handle_guest_page_faults(
    mutable: &mut Mutable,
    context: &mut Context,
    scause: u64,
    htval: u64,
    stval: u64,
) {
    let gpaddr = htval_to_gpaddr(htval, stval);
    let htinst = read_csr!("htinst");

    let fault_kind = match scause {
        SCAUSE_GUEST_INST_PAGE_FAULT => ExitPageFaultKind::Execute,
        SCAUSE_GUEST_LOAD_PAGE_FAULT => ExitPageFaultKind::Load,
        SCAUSE_GUEST_STORE_PAGE_FAULT => ExitPageFaultKind::Store,
        _ => unreachable!("Invalid guest page fault scause: {}", scause),
    };

    handle_guest_page_fault(mutable, context, htinst, gpaddr, fault_kind);
}

extern "C" fn vcpu_trap_handler(vcpu: *mut VCpu) -> ! {
    let context = unsafe { &mut (*vcpu).context };
    debug_assert_eq!(context.magic, CONTEXT_MAGIC);

    save_virtual_csrs(context);

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
    let scause_str = scause_to_string(is_intr, code);

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
            let (error, value) = match mutable.handle_sbi_call(context) {
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

            context.sepc += 4; // size of virtual instruction

            mutable.trigger_vm_exit(VCPU_EXIT_IDLE, ExitInfo::empty());
        }
        _ => {
            match scause {
                SCAUSE_GUEST_INST_PAGE_FAULT
                | SCAUSE_GUEST_LOAD_PAGE_FAULT
                | SCAUSE_GUEST_STORE_PAGE_FAULT => {
                    handle_guest_page_faults(&mut mutable, context, scause, htval, stval);
                }
                SCAUSE_SV_EXT_INTR => {
                    use super::plic::use_plic;
                    drop(mutable);

                    // FIXME: dup
                    use_plic(|plic| {
                        plic.handle_interrupt();
                    });
                    switch_thread();
                }
                _ => {
                    panic!(
                        "VM exit: {} (sepc={:x}, htval={:x}, stval={:x})",
                        scause_str, context.sepc, htval, stval
                    );
                }
            };
        }
    }

    drop(mutable);
    switch_thread();
}

#[unsafe(naked)]
pub extern "C" fn vcpu_trap_entry() -> ! {
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
    hideleg |= 1 << 10; // Supervisor external interrupt

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
