use core::arch::asm;

mod backtrace;
mod cpuvar;
mod csr;
mod interrupt;
mod sbi;
mod thread;

pub use backtrace::backtrace;
pub use cpuvar::cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
use csr::write_stvec;
use csr::TrapMode;
use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
pub use thread::yield_cpu;
pub use thread::Thread;

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, FtlError> {
    // Identical mapping.
    Ok(VAddr::from_nonzero(paddr.as_nonzero()))
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<VAddr, FtlError> {
    // Identical mapping.
    Ok(PAddr::from_nonzero(vaddr.as_nonzero()))
}

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn console_write(bytes: &[u8]) {
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}

pub fn init() {
    unsafe {
        write_stvec(
            interrupt::switch_to_kernel as *const () as usize,
            TrapMode::Direct,
        );

        // riscv::register::sie::set_sext();
        // write_sie(read_sie() | 1 << 9); // Supervisor External Interrupt Enable
    }
}
