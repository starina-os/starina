use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use starina::device_tree::Reg;
use starina_types::address::PAddr;
use starina_types::error::ErrorCode;
use starina_types::interrupt::Irq;

use crate::arch::get_cpuvar;
use crate::cpuvar::CpuId;
use crate::folio::Folio;
use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLock;
use crate::utils::fxhashmap::FxHashMap;
use crate::utils::mmio::LittleEndian;
use crate::utils::mmio::MmioFolio;
use crate::utils::mmio::MmioReg;
use crate::utils::mmio::ReadWrite;

const IRQ_MAX: usize = 1024;
const PLIC_SIZE: usize = 0x400000;

// Interrupt Source Priority
// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#3-interrupt-priorities
fn priority_reg(irq: Irq) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(4 * (irq.as_raw() as usize))
}

// Interrupt Enable Bits
// https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#5-interrupt-enables
fn enable_reg(irq: Irq) -> MmioReg<LittleEndian, ReadWrite, u32> {
    MmioReg::new(0x2080 + ((irq.as_raw() as usize) / 32 * size_of::<u32>()))
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

static PLIC: SpinLock<Option<Plic>> = SpinLock::new(None);

pub fn init(reg: &[Reg]) {
    let plic = Plic::new(reg);
    PLIC.lock().replace(plic);
}

pub fn use_plic(f: impl FnOnce(&mut Plic)) {
    let mut plic_lock = PLIC.lock();
    let plic = plic_lock.as_mut().expect("PLIC is not initialized");
    f(plic);
}

pub struct Plic {
    folio: MmioFolio,
    listeners: FxHashMap<Irq, SharedRef<Interrupt>>,
}

impl Plic {
    pub fn new(reg: &[Reg]) -> Self {
        debug_assert!(reg.len() == 1);

        let plic_paddr: usize = reg[0].addr as usize;

        trace!("PLIC: paddr={:#x}", plic_paddr);
        let folio = Folio::alloc_at(PAddr::new(plic_paddr), PLIC_SIZE).unwrap();
        let mmio_folio = MmioFolio::from_folio(folio).unwrap();

        Plic {
            folio: mmio_folio,
            listeners: FxHashMap::new(),
        }
    }

    pub fn init_per_cpu(&mut self, cpu_id: CpuId) {
        // Enable all interrupts by setting the threshold to 0.
        //
        // Note: Don't use cpuvar() here because it's not initialized yet.
        threshold_reg(cpu_id).write(&mut self.folio, 0);
    }

    pub fn get_pending_irq(&mut self) -> Irq {
        let raw_irq = claim_reg(get_cpuvar().cpu_id).read(&mut self.folio);
        Irq::from_raw(raw_irq)
    }

    pub fn enable_irq(&mut self, irq: Irq) {
        assert!((irq.as_raw() as usize) < IRQ_MAX);
        trace!("PLIC: enabling irq={}", irq.as_raw());
        trace!("PLIC priority: {:x}", 4 * (irq.as_raw() as usize));

        priority_reg(irq).write(&mut self.folio, 1);

        let enable = enable_reg(irq);
        let mut value = enable.read(&mut self.folio);
        value |= 1 << ((irq.as_raw() as usize) % 32);
        enable.write(&mut self.folio, value);
    }

    pub fn acknowledge(&mut self, irq: Irq) {
        assert!((irq.as_raw() as usize) < IRQ_MAX);

        claim_reg(get_cpuvar().cpu_id).write(&mut self.folio, irq.as_raw());
    }

    pub fn register_listener(&mut self, irq: Irq, listener: SharedRef<Interrupt>) {
        self.listeners.insert(irq, listener);
    }

    pub fn unregister_listener(&mut self, irq: Irq) {
        self.listeners.remove(&irq);
    }

    pub fn handle_interrupt(&mut self) {
        let irq = self.get_pending_irq();
        if let Some(listener) = self.listeners.get(&irq) {
            listener.trigger().unwrap();
        }
    }
}
