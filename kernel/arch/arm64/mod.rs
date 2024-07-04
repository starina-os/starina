use core::arch::asm;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;

mod backtrace;
mod cpuvar;
mod thread;

pub use backtrace::backtrace;
pub use cpuvar::cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
pub use thread::yield_cpu;
pub use thread::Thread;

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;

pub fn paddr2vaddr(paddr: PAddr) -> Option<VAddr> {
    // Identical mapping.
    Some(VAddr::from_nonzero(paddr.as_nonzero()))
}

pub fn vaddr2paddr(vaddr: VAddr) -> Option<PAddr> {
    // Identical mapping.
    Some(PAddr::from_nonzero(vaddr.as_nonzero()))
}

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn console_write(bytes: &[u8]) {
    let ptr: *mut u8 = 0x9000000 as *mut u8;
    for byte in bytes {
        unsafe {
            core::ptr::write_volatile(ptr, *byte);
        }
    }
}

pub fn init() {}
