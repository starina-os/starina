use alloc::string::String;
use core::arch::asm;

use starina::device_tree::Reg;
use starina::error::ErrorCode;
use starina::interrupt::Irq;
use starina::interrupt::IrqMatcher;

use super::plic;
use super::plic::use_plic;
use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;
use crate::thread::switch_thread;

pub extern "C" fn interrupt_handler() -> ! {
    let cpuvar = super::get_cpuvar();

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
        (false, 10) => "reserved",
        (false, 11) => "environment call from M-mode",
        (false, 12) => "instruction page fault",
        (false, 13) => "load page fault",
        (false, 15) => "store/AMO page fault",
        _ => "unknown",
    };

    let sepc = unsafe { (*cpuvar.arch.context).sepc } as u64;

    trace!(
        "interrupt: {} (scause={:#x}), sepc: {:#x}, stval: {:#x}",
        scause_str, scause, sepc, stval
    );

    if (is_intr, code) == (true, 9) {
        use_plic(|plic| {
            plic.handle_interrupt();
        });
        switch_thread();
    } else if (is_intr, code) == (false, 8) {
        unsafe {
            // Skip ecall instruction
            (*cpuvar.arch.context).sepc += 4;
        }

        // let a0 = unsafe { (*cpuvar.arch.context).a0 } as isize;
        // let a1 = unsafe { (*cpuvar.arch.context).a1 } as isize;
        // let a2 = unsafe { (*cpuvar.arch.context).a2 } as isize;
        // let a3 = unsafe { (*cpuvar.arch.context).a3 } as isize;
        // let a4 = unsafe { (*cpuvar.arch.context).a4 } as isize;
        // let a5 = unsafe { (*cpuvar.arch.context).a5 } as isize;
        // let ret = syscall_handler(a0, a1, a2, a3, a4, a5);
        todo!()
    } else {
        panic!("unhandled intrrupt");
    }
}
pub static INTERRUPT_CONTROLLER: PlicWrapper = PlicWrapper::new();

#[derive(Debug)]
pub enum InterruptCellParseError {
    InvalidCellCount,
}

pub struct PlicWrapper {
    _private: (),
}

impl PlicWrapper {
    pub const fn new() -> Self {
        Self { _private: () }
    }

    pub fn try_init(&self, compatible: &[String], reg: &[Reg]) -> Result<(), ErrorCode> {
        plic::try_init(compatible, reg)
    }

    pub fn parse_interrupts_cell(
        &self,
        interrupts_cell: &[u32],
    ) -> Result<IrqMatcher, InterruptCellParseError> {
        if interrupts_cell.len() != 1 {
            return Err(InterruptCellParseError::InvalidCellCount);
        }

        let irq = Irq::from_raw(interrupts_cell[0]);
        Ok(IrqMatcher::Static(irq))
    }

    pub fn acquire_irq(&self, irq_matcher: IrqMatcher) -> Result<Irq, ErrorCode> {
        match irq_matcher {
            IrqMatcher::Static(irq) => Ok(irq),
        }
    }

    pub fn enable_irq(&self, interrupt: SharedRef<Interrupt>) {
        use_plic(|plic| {
            plic.enable_irq(interrupt.irq());
            plic.register_listener(interrupt.irq(), interrupt);
        });
    }

    pub fn acknowledge_irq(&self, irq: Irq) {
        use_plic(|plic| {
            plic.acknowledge(irq);
        });
    }
}
