use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
use ftl_types::interrupt::Irq;

use crate::cpuvar::CpuId;
use crate::interrupt::Interrupt;
use crate::ref_counted::SharedRef;

pub fn halt() -> ! {
    todo!()
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, FtlError> {
    todo!()
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, FtlError> {
    todo!()
}

pub fn console_write(bytes: &[u8]) {
    todo!()
}

pub fn backtrace<F>(mut callback: F)
where
    F: FnMut(usize),
{
    todo!()
}

pub fn return_to_user() -> ! {
    todo!()
}

pub unsafe extern "C" fn kernel_syscall_entry(
    _a0: isize,
    _a1: isize,
    _a2: isize,
    _a3: isize,
    _a4: isize,
    _a5: isize,
    _a6: isize,
) -> isize {
    todo!()
}

pub fn set_cpuvar(cpuvar: *mut crate::cpuvar::CpuVar) {
    todo!()
}

pub fn cpuvar() -> &'static crate::cpuvar::CpuVar {
    todo!()
}

pub fn create_interrupt(interrupt: &SharedRef<Interrupt>) -> Result<(), FtlError> {
    todo!()
}

pub fn ack_interrupt(irq: Irq) -> Result<(), FtlError> {
    todo!()
}

pub fn init(cpu_id: CpuId, device_tree: &crate::device_tree::DeviceTree) {
    todo!()
}

pub struct CpuVar {}

impl CpuVar {
    pub fn new(idle_thread: &SharedRef<crate::thread::Thread>) -> Self {
        todo!()
    }
}

pub struct VmSpace {}

impl VmSpace {
    pub fn new() -> Result<VmSpace, FtlError> {
        todo!()
    }

    pub fn map_fixed(&self, vaddr: VAddr, paddr: PAddr, len: usize) -> Result<(), FtlError> {
        todo!()
    }

    pub fn map_anywhere(&self, paddr: PAddr, len: usize) -> Result<VAddr, FtlError> {
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

    pub fn new_kernel(
        vmspace: SharedRef<crate::vmspace::VmSpace>,
        pc: usize,
        sp: usize,
        arg: usize,
    ) -> Thread {
        todo!()
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;
pub const USERSPACE_START: VAddr = VAddr::new(0);
pub const USERSPACE_END: VAddr = VAddr::new(0);
