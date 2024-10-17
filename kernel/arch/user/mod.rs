use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::cpuvar::CpuId;
use crate::interrupt::Interrupt;
use crate::refcount::SharedRef;
use crate::syscall::syscall_handler;

pub fn halt() -> ! {
    todo!()
}

pub fn paddr2vaddr(_paddr: PAddr) -> Result<VAddr, FtlError> {
    todo!()
}

pub fn vaddr2paddr(_vaddr: VAddr) -> Result<PAddr, FtlError> {
    todo!()
}

pub fn console_write(_bytes: &[u8]) {
    todo!()
}

pub fn backtrace<F>(_callback: F)
where
    F: FnMut(usize),
{
    todo!()
}

pub fn return_to_user(_thread: *mut Thread, _sysret: Option<isize>) -> ! {
    todo!()
}

pub fn idle() -> ! {
    todo!()
}

pub unsafe extern "C" fn kernel_syscall_entry(
    a0: isize,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    n: isize,
) -> isize {
    // Dummy if statement to avoid unused variable warnings.
    if false {
        syscall_handler(a0, a1, a2, a3, a4, n);
    }

    todo!();
}

pub fn set_cpuvar(_cpuvar: *mut crate::cpuvar::CpuVar) {
    todo!()
}

pub fn get_cpuvar() -> &'static crate::cpuvar::CpuVar {
    todo!()
}

pub fn interrupt_create(_interrupt: &SharedRef<Interrupt>) -> Result<(), FtlError> {
    todo!()
}

pub fn interrupt_ack(_irq: Irq) -> Result<(), FtlError> {
    todo!()
}

pub fn early_init(_cpu_id: CpuId) {
    todo!()
}

pub fn init(_cpu_id: CpuId, _device_tree: Option<&crate::device_tree::DeviceTree>) {
    todo!()
}

pub struct CpuVar {}

impl CpuVar {
    pub fn new(_idle_thread: &SharedRef<crate::thread::Thread>) -> Self {
        todo!()
    }
}

pub struct VmSpace {}

impl VmSpace {
    pub fn new() -> Result<VmSpace, FtlError> {
        todo!()
    }

    pub fn map_fixed(&self, _vaddr: VAddr, _paddr: PAddr, _len: usize) -> Result<(), FtlError> {
        todo!()
    }

    pub fn map_anywhere(&self, _paddr: PAddr, _len: usize) -> Result<VAddr, FtlError> {
        todo!()
    }

    pub fn switch(&self) {
        todo!()
    }
}

pub struct Thread {}

impl Thread {
    pub fn new_idle() -> Thread {
        todo!()
    }

    pub fn new_kernel(_pc: usize, _arg: usize) -> Thread {
        todo!()
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;
pub const USERSPACE_START: VAddr = VAddr::new(0);
pub const USERSPACE_END: VAddr = VAddr::new(0);
