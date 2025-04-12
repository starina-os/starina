#![allow(unused_variables)]

use std::cell::RefCell;
use std::io::Write;

use starina::address::DAddr;
use starina::address::PAddr;
use starina::address::VAddr;
use starina::device_tree::Reg;
use starina::error::ErrorCode;
use starina::interrupt::Irq;
use starina::interrupt::IrqMatcher;
use starina_types::syscall::RetVal;
use starina_types::vmspace::PageProtect;

use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;

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

// #[naked]
pub extern "C" fn enter_kernelland(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
) -> RetVal {
    todo!()
}

pub fn enter_userland(thread: *mut crate::arch::Thread) -> ! {
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

pub fn map_daddr(paddr: PAddr) -> Result<DAddr, ErrorCode> {
    todo!()
}

pub fn unmap_daddr(daddr: DAddr) -> Result<(), ErrorCode> {
    todo!()
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, ErrorCode> {
    todo!()
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, ErrorCode> {
    todo!()
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
