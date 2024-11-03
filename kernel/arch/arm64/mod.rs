use core::arch::asm;
use core::arch::global_asm;

use starina_types::address::PAddr;
use starina_types::address::VAddr;

mod backtrace;
mod cpuvar;
mod gic_v2;
mod idle;
mod thread;
mod vmspace;

global_asm!(include_str!("interrupt.S"));

pub use backtrace::backtrace;
pub use cpuvar::cpuvar as get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
pub use gic_v2::interrupt_ack;
pub use gic_v2::interrupt_create;
pub use idle::idle;
use starina_types::error::FtlError;
pub use thread::kernel_syscall_entry;
pub use thread::return_to_user;
pub use thread::Thread;
pub use vmspace::VmSpace;
pub use vmspace::USERSPACE_END;
pub use vmspace::USERSPACE_START;

use crate::cpuvar::CpuId;
use crate::device_tree::DeviceTree;

pub const VSYSCALL_ENTRY_ADDR: VAddr = VAddr::new(0x9000);
pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, FtlError> {
    // Identical mapping.
    Ok(VAddr::new(paddr.as_usize()))
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, FtlError> {
    // Identical mapping.
    Ok(PAddr::new(vaddr.as_usize()))
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

extern "C" {
    pub static arm64_exception_vector: [u8; 128 * 16];
}

pub fn early_init(_cpu_id: CpuId) {
    unsafe {
        asm!("msr vbar_el1, {}", in(reg) &arm64_exception_vector as *const _ as u64);

        // Enable d-cache and i-cache.
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
        sctlr |= (1 << 2) | (1 << 12);
        asm!("msr sctlr_el1, {}", in(reg) sctlr);
    }
}

pub fn init(_cpu_id: CpuId, device_tree: Option<&DeviceTree>) {
    gic_v2::init(device_tree.unwrap());
}
