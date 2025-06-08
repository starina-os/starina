#![allow(unused_variables)]

use std::cell::RefCell;
use std::io::Write;

use starina::address::PAddr;
use starina::address::Paddr;
use starina::address::VAddr;
use starina::device_tree::Reg;
use starina::error::ErrorCode;
use starina::interrupt::Irq;
use starina::interrupt::IrqMatcher;
use starina_types::syscall::RetVal;
use starina_types::vmspace::PageProtect;

use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;

mod timer;

pub use timer::get_monotonic_time;
pub use timer::schedule_timer_interrupt;
pub use timer::set_timer_frequency;

pub const PAGE_SIZE: usize = 4096;

pub fn percpu_init() {
    todo!()
}

pub fn halt() -> ! {
    panic!("halted");
}

pub fn console_write(s: &[u8]) {
    std::io::stdout().write_all(s).unwrap();
}

// #[unsafe(naked)]
pub extern "C" fn inkernel_syscall_entry(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
) -> RetVal {
    todo!()
}

pub fn user_entry(thread: *mut crate::arch::Thread) -> ! {
    todo!()
}

pub fn idle() -> ! {
    todo!();
}

pub struct Thread {}

impl Thread {
    pub fn new_inkernel(pc: usize, arg: usize) -> Thread {
        Thread {}
    }

    pub fn new_idle() -> Thread {
        Thread {}
    }

    pub fn set_retval(&mut self, retval: RetVal) {
        todo!()
    }
}

pub struct CpuVar {}

impl CpuVar {
    pub fn new(idle_thread: &crate::refcount::SharedRef<crate::thread::Thread>) -> Self {
        CpuVar {}
    }
}

thread_local! {
    static CPUVAR: RefCell<*mut crate::cpuvar::CpuVar> =
        const { RefCell::new(std::ptr::null_mut()) }
    ;
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    CPUVAR.with_borrow_mut(|cpuvar_ref| {
        *cpuvar_ref = cpuvar;
    });
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    CPUVAR.with_borrow(|cpuvar_ref| {
        debug_assert!(!cpuvar_ref.is_null());
        unsafe { &**cpuvar_ref }
    })
}

pub struct VmSpace {}

impl VmSpace {
    pub fn new() -> Result<VmSpace, ErrorCode> {
        todo!()
    }

    pub fn map_fixed(
        &self,
        vaddr: VAddr,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<(), ErrorCode> {
        todo!()
    }

    pub fn map_anywhere(
        &self,
        paddr: PAddr,
        len: usize,
        prot: PageProtect,
    ) -> Result<VAddr, ErrorCode> {
        todo!()
    }

    pub fn switch(&self) {
        todo!()
    }
}

pub fn map_paddr(paddr: PAddr) -> Result<Paddr, ErrorCode> {
    todo!()
}

pub fn unmap_paddr(paddr: Paddr) -> Result<(), ErrorCode> {
    todo!()
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, ErrorCode> {
    todo!()
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, ErrorCode> {
    todo!()
}

pub fn find_free_ram<F>(paddr: PAddr, size: usize, callback: F)
where
    F: Fn(PAddr, usize),
{
    // For host, just call the callback with the original parameters
    callback(paddr, size);
}

pub static INTERRUPT_CONTROLLER: InterruptController = InterruptController {};

pub struct InterruptController {}

impl InterruptController {
    pub const fn new() -> Self {
        todo!()
    }

    pub fn try_init(&self, compatible: &[String], reg: &[Reg]) -> Result<(), ErrorCode> {
        todo!()
    }

    pub fn parse_interrupts_cell(&self, interrupts_cell: &[u32]) -> Result<IrqMatcher, ()> {
        todo!()
    }
    pub fn acquire_irq(&self, irq_matcher: IrqMatcher) -> Result<Irq, ErrorCode> {
        todo!()
    }

    pub fn enable_irq(&self, interrupt: SharedRef<Interrupt>) {
        todo!()
    }

    pub fn acknowledge_irq(&self, irq: Irq) {
        todo!()
    }
}
