use core::arch::asm;
use core::arch::global_asm;

use ftl_types::address::PAddr;
use ftl_types::address::VAddr;

mod backtrace;
mod cpuvar;
mod gic_v2;
mod thread;

global_asm!(include_str!("interrupt.S"));

pub use backtrace::backtrace;
pub use cpuvar::cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
pub use gic_v2::ack_interrupt;
pub use gic_v2::create_interrupt;
pub use thread::yield_cpu;
pub use thread::Thread;

use crate::device_tree::DeviceTree;

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

pub fn idle() -> ! {
    loop {
        yield_cpu(); // FIXME:

        unsafe {
            asm!("msr daifclr, #2");
            asm!("nop"); // FIXME: use wfi
            asm!("msr daifset, #2");
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

#[no_mangle]
extern "C" fn arm64_handle_exception() {
    panic!("unhandled exception");
}

#[no_mangle]
extern "C" fn handle_syscall() {
    panic!("handle_syscall");
}

#[no_mangle]
extern "C" fn arm64_handle_interrupt() {
    gic_v2::handle_interrupt();
}

extern "C" {
    static arm64_exception_vector: [u8; 128 * 16];
}

pub fn init(device_tree: &DeviceTree) {
    unsafe {
        asm!("msr vbar_el1, {}", in(reg) &arm64_exception_vector as *const _ as u64);
    }

    gic_v2::init(device_tree);
}
