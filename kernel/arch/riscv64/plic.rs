use alloc::collections::BTreeMap;

use ftl_types::address::PAddr;
use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::arch::get_cpuvar;
use crate::cpuvar::CpuId;
use crate::device_tree::DeviceTree;
use crate::folio::Folio;
use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::utils::mmio::LittleEndian;
use crate::utils::mmio::MmioFolio;
use crate::utils::mmio::MmioReg;
use crate::utils::mmio::ReadWrite;

const IRQ_MAX: usize = 1024;
const PLIC_SIZE: usize = 0x400000;

// Interrupt Source Priority
// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#3-interrupt-priorities
fn priority_reg(irq: Irq) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(4 * irq.as_usize())
}

// Interrupt Enable Bits
// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#5-interrupt-enables
fn enable_reg(irq: Irq) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(0x2080 + (irq.as_usize() / 32 * size_of::<u32>()))
}

/// Interrupt Claim Register
/// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#7-interrupt-claim-process
fn claim_reg(hart: CpuId) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(0x201004 + 0x2000 * hart.as_usize())
}

// Priority Threshold
// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#6-priority-thresholds
fn threshold_reg(hart: CpuId) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(0x201000 + 0x2000 * hart.as_usize())
}

struct Plic {
    folio: MmioFolio,
}

impl Plic {
    pub fn new(folio: MmioFolio) -> Plic {
        Plic { folio }
    }

    pub fn init_per_cpu(&mut self, cpu_id: CpuId) {
        // Enable all interrupts by setting the threshold to 0.
        //
        // Note: Don't use cpuvar() here because it's not initialized yet.
        threshold_reg(cpu_id).write(&mut self.folio, 0);
    }

    pub fn get_pending_irq(&mut self) -> Irq {
        let raw_irq = claim_reg(get_cpuvar().cpu_id).read(&mut self.folio);
        Irq::from_raw(raw_irq as usize)
    }

    pub fn enable_irq(&mut self, irq: Irq) {
        assert!(irq.as_usize() < IRQ_MAX);

        priority_reg(irq).write(&mut self.folio, 1);

        let enable = enable_reg(irq);
        let mut value = enable.read(&mut self.folio);
        value |= 1 << (irq.as_usize() % 32);
        enable.write(&mut self.folio, value);
    }

    pub fn ack_interrupt(&mut self, irq: Irq) {
        assert!(irq.as_usize() < IRQ_MAX);

        claim_reg(get_cpuvar().cpu_id).write(&mut self.folio, irq.as_usize() as u32);
    }
}

static PLIC: SpinLock<Option<Plic>> = SpinLock::new(None);
static LISTENERS: SpinLock<BTreeMap<Irq, SharedRef<Interrupt>>> = SpinLock::new(BTreeMap::new());

pub fn interrupt_create(interrupt: &SharedRef<Interrupt>) -> Result<(), FtlError> {
    let irq = interrupt.irq();
    PLIC.lock().as_mut().unwrap().enable_irq(irq);
    LISTENERS.lock().insert(irq, interrupt.clone());
    Ok(())
}

pub fn interrupt_ack(irq: Irq) -> Result<(), FtlError> {
    PLIC.lock().as_mut().unwrap().ack_interrupt(irq);
    Ok(())
}

pub fn handle_interrupt() {
    let irq = PLIC.lock().as_mut().unwrap().get_pending_irq();
    let listeners = LISTENERS.lock();
    if let Some(listener) = listeners.get(&irq) {
        listener.trigger().unwrap();
    }
}

pub fn init(cpu_id: CpuId, device_tree: &DeviceTree) {
    let plic_paddr: usize = device_tree
        .find_device_by_id("sifive,plic-1.0.0")
        .unwrap()
        .reg as usize;

    trace!("PLIC: paddr={:#x}", plic_paddr);
    let folio = Folio::alloc_fixed(PAddr::new(plic_paddr), PLIC_SIZE).unwrap();
    let mut plic = Plic::new(MmioFolio::from_folio(folio).unwrap());
    plic.init_per_cpu(cpu_id);

    PLIC.lock().replace(plic);
}
