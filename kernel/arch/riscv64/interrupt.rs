use alloc::string::String;

use starina::interrupt::Irq;

pub static INTERRUPT_CONTROLLER: spin::Lazy<InterruptController> =
    spin::Lazy::new(InterruptController::new);

#[derive(Debug)]
pub enum InterruptCellParseError {
    InvalidCellCount,
}

pub struct InterruptController {
    pub irq_base: u32,
}

impl InterruptController {
    pub fn new() -> Self {
        Self { irq_base: 0 }
    }

    pub fn is_compatible(&self, compatible: &[String]) -> bool {
        compatible.iter().any(|s| s == "riscv,plic0")
    }

    pub fn interrupts_cell_to_irq(
        &self,
        interrupts_cell: &[u32],
    ) -> Result<Irq, InterruptCellParseError> {
        if interrupts_cell.len() != 1 {
            return Err(InterruptCellParseError::InvalidCellCount);
        }

        Ok(Irq::new(interrupts_cell[0]))
    }

    pub fn enable_irq(&self, irq: Irq) {
        todo!()
    }
}
