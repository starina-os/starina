use core::arch::asm;

use ftl_types::address::PAddr;

use crate::folio::Folio;
use crate::spinlock::SpinLock;
use crate::utils::mmio::LittleEndian;
use crate::utils::mmio::MmioFolio;
use crate::utils::mmio::MmioReg;
use crate::utils::mmio::ReadWrite;

const TPR_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x80);
const LOGICAL_DEST_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0xd0);
const DEST_FORMAT_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0xe0);
const EOI_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0xb0);
const SPURIOUS_INT_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0xf0);
const LVT_TIMER_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x320);
const LVT_ERROR_REG: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x370);
const IA32_APIC_BASE_MSR: u32 = 0x1b;

pub static LOCAL_APIC: SpinLock<Option<LocalApic>> = SpinLock::new(None);

pub struct LocalApic {
    folio: MmioFolio,
}

impl LocalApic {
    pub fn new(folio: Folio) -> LocalApic {
        LocalApic {
            folio: MmioFolio::from_folio(folio).unwrap(),
        }
    }

    pub fn init(&mut self) {
        // Enable APIC.
        {
            let apic_base = self.folio.paddr().as_usize() as u64;
            let value = (apic_base & 0xfffff100) | 0x0800;
            unsafe {
                asm!(
                    "wrmsr",
                    in("ecx") IA32_APIC_BASE_MSR,
                    in("eax") (value & 0xffffffff) as u32,
                    in("edx") ((value >> 32) & 0xffffffff) as u32,
                    options(nostack),
                )
            }
        }

        TPR_REG.write(&mut self.folio, 0);
        SPURIOUS_INT_REG.write(&mut self.folio, 1 << 8);
        LOGICAL_DEST_REG.write(&mut self.folio, 0x01000000);
        DEST_FORMAT_REG.write(&mut self.folio, 0xffff_ffff);
        LVT_TIMER_REG.write(&mut self.folio, 1 << 16 /* masked (disabled) */);
        LVT_ERROR_REG.write(&mut self.folio, 1 << 16 /* masked (disabled) */);
    }

    pub fn ack_interrupt(&mut self) {
        EOI_REG.write(&mut self.folio, 0);
    }
}

pub fn ack_interrupt() {
    LOCAL_APIC.lock().as_mut().unwrap().ack_interrupt();
}

pub fn init(paddr: PAddr) {
    let folio = Folio::alloc_fixed(paddr, 0x1000).unwrap();
    let mut lapic = LocalApic::new(folio);
    lapic.init();

    // Clear any pending interrupts.
    lapic.ack_interrupt();

    LOCAL_APIC.lock().replace(lapic);
}
