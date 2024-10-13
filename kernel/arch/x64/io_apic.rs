use ftl_types::address::PAddr;
use ftl_types::interrupt::Irq;

use super::idt::VECTOR_IRQ_BASE;
use crate::folio::Folio;
use crate::spinlock::SpinLock;
use crate::utils::mmio::LittleEndian;
use crate::utils::mmio::MmioFolio;
use crate::utils::mmio::MmioReg;
use crate::utils::mmio::ReadWrite;

const IOREGSEL_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x00);
const IOWIN_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x10);

struct IoApicReg(u8);

impl IoApicReg {
    fn write(&self, folio: &mut MmioFolio, value: u32) {
        IOREGSEL_REG.write(folio, self.0 as u32);
        IOWIN_REG.write(folio, value);
    }

    fn read(&self, folio: &mut MmioFolio) -> u32 {
        IOREGSEL_REG.write(folio, self.0 as u32);
        IOWIN_REG.read(folio)
    }
}

const IOAPIC_REG_IOAPICVER: IoApicReg = IoApicReg(0x01);

fn ioredtbl_low_reg(irq: Irq) -> IoApicReg {
    IoApicReg(0x10 + (2 * irq.as_usize() as u8))
}

fn ioredtbl_high_reg(irq: Irq) -> IoApicReg {
    IoApicReg(0x10 + (2 * irq.as_usize() as u8 + 1))
}

pub static IO_APIC: SpinLock<Option<IoApic>> = SpinLock::new(None);

pub struct IoApic {
    folio: MmioFolio,
}

impl IoApic {
    pub fn new(folio: Folio) -> IoApic {
        IoApic {
            folio: MmioFolio::from_folio(folio).unwrap(),
        }
    }

    pub fn init(&mut self) {
        // Disable all hardware interrupts.
        let n = IOAPIC_REG_IOAPICVER.read(&mut self.folio) >> 16 + 1;
        for i in 0..n {
            let irq = Irq::from_raw(i as usize);
            ioredtbl_high_reg(irq).write(&mut self.folio, 0);
            ioredtbl_low_reg(irq).write(&mut self.folio, 1 << 16 /* masked */);
        }
    }

    pub fn enable_irq(&mut self, irq: Irq) {
        ioredtbl_high_reg(irq).write(&mut self.folio, 0);
        ioredtbl_low_reg(irq).write(&mut self.folio, VECTOR_IRQ_BASE + irq.as_usize() as u32);
    }
}

pub fn init(paddr: PAddr) {
    let folio = Folio::alloc_fixed(paddr, 0x1000).unwrap();
    let mut ioapic = IoApic::new(folio);
    ioapic.init();

    IO_APIC.lock().replace(ioapic);
}
