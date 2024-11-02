use core::arch::asm;

use csr::write_stvec;
use csr::StvecMode;
use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::FtlError;
use switch::switch_to_kernel;

use crate::cpuvar::CpuId;

mod backtrace;
mod cpuvar;
mod csr;
mod idle;
mod interrupt;
mod plic;
mod sbi;
mod switch;
mod thread;
mod vmspace;

pub use backtrace::backtrace;
pub use cpuvar::get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
pub use idle::idle;
pub use plic::interrupt_ack;
pub use plic::interrupt_create;
pub use switch::kernel_syscall_entry;
pub use switch::return_to_user;
pub use thread::Thread;
pub use vmspace::VmSpace;
pub use vmspace::USERSPACE_END;
pub use vmspace::USERSPACE_START;

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
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}

pub fn early_init(_cpu_id: CpuId) {}

pub fn init(cpu_id: CpuId, device_tree: Option<&crate::device_tree::DeviceTree>) {
    unsafe {
        let mut sie: u64;
        asm!("csrr {}, sie", out(reg) sie);
        sie |= 1 << 1; // SSIE: supervisor-level software interrupts
        sie |= 1 << 5; // STIE: supervisor-level timer interrupts
        sie |= 1 << 9; // SEIE: supervisor-level external interrupts
        asm!("csrw sie, {}", in(reg) sie);

        let mut sstatus: u64;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        sstatus |= 1 << 18; // SUM: Supervisor User Memory access
        asm!("csrw sstatus, {}", in(reg) sstatus);

        write_stvec(switch_to_kernel as *const () as usize, StvecMode::Direct);

        // TODO: Make sure cpuvar is already initialized.
        asm!("csrw sscratch, tp");
    }

    let device_tree = device_tree.unwrap();
    plic::init(cpu_id, device_tree);
}
