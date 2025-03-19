use alloc::string::String;

use starina::device_tree::Reg;
use starina::error::ErrorCode;
use starina::interrupt::Irq;

use super::plic::use_plic;
use super::plic::{self};

pub static INTERRUPT_CONTROLLER: spin::Lazy<InterruptController> =
    spin::Lazy::new(InterruptController::new);

#[derive(Debug)]
pub enum InterruptCellParseError {
    InvalidCellCount,
}

// FIXME: Move this to plic.rs
pub struct InterruptController {}

impl InterruptController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn try_init(&self, compatible: &[String], reg: &[Reg]) -> Result<(), ErrorCode> {
        if !compatible.iter().any(|s| s == "riscv,plic0") {
            return Err(ErrorCode::NotSupported);
        }

        plic::init(reg);
        Ok(())
    }

    pub fn interrupts_cell_to_irq(
        &self,
        interrupts_cell: &[u32],
    ) -> Result<Irq, InterruptCellParseError> {
        if interrupts_cell.len() != 1 {
            return Err(InterruptCellParseError::InvalidCellCount);
        }

        Ok(Irq::from_raw(interrupts_cell[0]))
    }

    pub fn enable_irq(&self, irq: Irq) {
        use_plic(|plic| {
            plic.enable_irq(irq);
        });
    }

    pub fn acknowledge_irq(&self, irq: Irq) {
        use_plic(|plic| {
            plic.acknowledge(irq);
        });
    }
}
