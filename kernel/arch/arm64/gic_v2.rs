// > In the GIC architecture, all registers that are halfword-accessible or
// > byte-accessible use a little endian memory order model.
// >
// > 4.1.4 GIC register access

use alloc::collections::BTreeMap;

use ftl_types::address::PAddr;
use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::device_tree::DeviceTree;
use crate::folio::Folio;
use crate::interrupt::Interrupt;
use crate::ref_counted::SharedRef;
use crate::spinlock::SpinLock;
use crate::utils::mmio::LittleEndian;
use crate::utils::mmio::MmioFolio;
use crate::utils::mmio::MmioReg;
use crate::utils::mmio::ReadOnly;
use crate::utils::mmio::ReadWrite;
use crate::utils::mmio::WriteOnly;

/// Distributor Control Register.
const GICD_CTLR: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x000);
/// Interrupt Controller Type Register.
const GICD_TYPER: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x004);
/// Interrupt Set-Enable Registers.
#[allow(non_upper_case_globals)]
const GICD_IENABLERn: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x100);
/// Interrupt Priority Registers.
#[allow(non_upper_case_globals)]
const GICD_IPRIORITYRn: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x400);
/// Interrupt Processor Targets Registers.
#[allow(non_upper_case_globals)]
const GICD_ITARGETSRn: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x800);
/// CPU Interface Control Register,
const GICC_CTLR: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x000);
/// Interrupt Priority Mask Register.
const GICC_PMR: MmioReg<LittleEndian, ReadWrite, u32> = MmioReg::new(0x004);
/// Interrupt Acknowledge Register.
const GICC_IAR: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x00c);
/// End of Interrupt Register.
const GICC_EOIR: MmioReg<LittleEndian, WriteOnly, u32> = MmioReg::new(0x010);

struct Gic {
    dist_folio: MmioFolio,
    cpu_folio: MmioFolio,
    num_max_intrs: usize,
}

impl Gic {
    pub fn init_device(mut dist_folio: MmioFolio, mut cpu_folio: MmioFolio) -> Self {
        // Reset the device.
        GICD_CTLR.write(&mut dist_folio, 0);
        // Determine the maximum number of interrupts (ITLinesNumber field).
        let it_lines_number = GICD_TYPER.read(&mut dist_folio) & 0b1111;
        let num_max_intrs = (it_lines_number + 1) * 32;

        GICC_PMR.write(&mut cpu_folio, 255);

        GICC_CTLR.write(&mut cpu_folio, 1);
        GICD_CTLR.write(&mut dist_folio, 1);

        Self {
            dist_folio,
            cpu_folio,
            num_max_intrs: num_max_intrs as usize,
        }
    }

    pub fn enable_irq(&mut self, irq: usize) {
        debug_assert!(irq < self.num_max_intrs);

        let irq_shift = (irq % 4) * 8;

        // Enable the interrupt.
        {
            let offset = irq / 32;
            let mut value = GICD_IENABLERn.read_with_offset(&mut self.dist_folio, offset);
            value |= 1 << (irq % 32);
            GICD_IENABLERn.write_with_offset(&mut self.dist_folio, offset, value);
        }

        // Set the priority of the interrupt to the highest.
        {
            let offset = irq / 4;
            let mut value = GICD_IPRIORITYRn.read_with_offset(&mut self.dist_folio, offset);
            value &= !(0xff << irq_shift);
            GICD_IPRIORITYRn.write_with_offset(&mut self.dist_folio, offset, value);
        }

        // Set the target processor to the first processor.
        // TODO: Multi-processor support.
        {
            let target = 0; /* CPU interface 0 */
            let offset = irq / 4;
            let mut value = GICD_ITARGETSRn.read_with_offset(&mut self.dist_folio, offset);
            value &= !(0xff << irq_shift);
            value |= (1 << target) << irq_shift;
            GICD_ITARGETSRn.write_with_offset(&mut self.dist_folio, offset, value);
        }
    }

    pub fn get_pending_irq(&mut self) -> usize {
        (GICC_IAR.read(&mut self.cpu_folio) & 0x3ff) as usize
    }

    pub fn ack_irq(&mut self, irq: usize) {
        debug_assert!(irq & !0x3ff == 0);
        GICC_EOIR.write(&mut self.cpu_folio, irq as u32);
    }
}

static GIC: SpinLock<Option<Gic>> = SpinLock::new(None);
static LISTENERS: SpinLock<BTreeMap<Irq, SharedRef<Interrupt>>> = SpinLock::new(BTreeMap::new());

pub fn create_interrupt(interrupt: &SharedRef<Interrupt>) -> Result<(), FtlError> {
    let irq = interrupt.irq();
    GIC.lock().as_mut().unwrap().enable_irq(irq.as_usize());
    LISTENERS.lock().insert(irq, interrupt.clone());
    Ok(())
}

pub fn ack_interrupt(irq: Irq) -> Result<(), FtlError> {
    GIC.lock().as_mut().unwrap().ack_irq(irq.as_usize());
    Ok(())
}

pub fn handle_interrupt() {
    let irq = GIC.lock().as_mut().unwrap().get_pending_irq();
    let irq = Irq::from_raw(irq);
    let listeners = LISTENERS.lock();
    if let Some(listener) = listeners.get(&irq) {
        listener.trigger().unwrap();
    }
}

pub fn init(device_tree: &DeviceTree) {
    let gicd_paddr: usize = device_tree
        .find_device_by_id("arm,cortex-a15-gic")
        .unwrap()
        .reg as usize;
    let gicd_folio = Folio::alloc_mmio(PAddr::new(gicd_paddr).unwrap(), 0x1000).unwrap();
    let gicc_folio = Folio::alloc_mmio(
        PAddr::new(gicd_paddr + 0x10000 /* FIXME: */).unwrap(),
        0x1000,
    )
    .unwrap();

    GIC.lock().replace(Gic::init_device(
        MmioFolio::from_folio(gicd_folio).unwrap(),
        MmioFolio::from_folio(gicc_folio).unwrap(),
    ));
}
