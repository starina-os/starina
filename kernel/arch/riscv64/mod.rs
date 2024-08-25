use core::arch::asm;
use core::arch::global_asm;

mod backtrace;
mod cpuvar;
mod csr;
mod interrupt;
mod plic;
mod sbi;
mod thread;
mod vmspace;

pub use backtrace::backtrace;
pub use cpuvar::cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
use csr::write_stvec;
use csr::TrapMode;
use ftl_types::address::PAddr;
use ftl_types::address::VAddr;
use ftl_types::error::FtlError;
pub use plic::ack_interrupt;
pub use plic::create_interrupt;
pub use thread::yield_cpu;
pub use thread::Thread;
pub use vmspace::VmSpace;

use crate::cpuvar::CpuId;

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, FtlError> {
    // Identical mapping.
    Ok(VAddr::from_nonzero(paddr.as_nonzero()))
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, FtlError> {
    // Identical mapping.
    Ok(PAddr::from_nonzero(vaddr.as_nonzero()))
}

global_asm!(r#"
.text
.global do_idle, __wfi_point
do_idle:
    fence
    csrsi sstatus, 1 << 1
__wfi_point:
    wfi
    csrci sstatus, 1 << 1
    ret
"#);

extern "C" {
    fn do_idle();
    pub static __wfi_point: u8;
}

pub fn idle() -> ! {
    loop {
        yield_cpu();
        unsafe {
            do_idle();
        }
    }
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

pub fn init(cpu_id: CpuId, device_tree: &crate::device_tree::DeviceTree) {
    extern "C" {
        fn switch_to_kernel();
    }

    unsafe {
        let mut sie: u64;
        asm!("csrr {}, sie", out(reg) sie);
        sie |= 1 << 1; // SSIE: supervisor-level software interrupts
        sie |= 1 << 5; // STIE: supervisor-level timer interrupts
        sie |= 1 << 9; // SEIE: supervisor-level external interrupts
        asm!("csrw sie, {}", in(reg) sie);

        write_stvec(switch_to_kernel as *const () as usize, TrapMode::Direct);

        // riscv::register::sie::set_sext();
        // write_sie(read_sie() | 1 << 9); // Supervisor External Interrupt Enable
    }

    plic::init(cpu_id, device_tree);
}
