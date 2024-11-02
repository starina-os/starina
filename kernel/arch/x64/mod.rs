#![allow(unused)]
use core::arch::asm;
use core::arch::global_asm;
use core::mem::offset_of;

use starina_types::address::PAddr;
use starina_types::address::VAddr;
use starina_types::error::FtlError;
use thread::Context;

use crate::cpuvar::CpuId;
use crate::refcount::SharedRef;

mod cpuvar;
mod gdt;
mod idle;
mod idt;
mod init;
mod interrupt;
mod io_apic;
mod local_apic;
mod mptable;
mod pic;
mod serial;
mod switch;
mod thread;
mod tss;
mod vmspace;

pub use cpuvar::get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use cpuvar::CpuVar;
pub use idle::idle;
pub use init::early_init;
pub use init::init;
pub use interrupt::interrupt_ack;
pub use interrupt::interrupt_create;
pub use switch::kernel_syscall_entry;
pub use switch::return_to_user;
pub use thread::Thread;
pub use vmspace::VmSpace;
pub use vmspace::USERSPACE_END;
pub use vmspace::USERSPACE_START;

const KERNEL_BASE: usize = 0xffff_8000_0000_0000;

pub fn halt() -> ! {
    warn!("entering halt");
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, FtlError> {
    Ok(VAddr::new(paddr.as_usize() + KERNEL_BASE))
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, FtlError> {
    Ok(PAddr::new(vaddr.as_usize() - KERNEL_BASE))
}

pub fn console_write(bytes: &[u8]) {
    for ch in bytes {
        serial::SERIAL0.print_char(*ch);
    }
}

pub fn backtrace<F>(mut callback: F)
where
    F: FnMut(usize),
{
    let mut rbp: usize;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }

    while rbp < KERNEL_BASE {
        let rip = unsafe { *(rbp as *const usize).offset(1) };
        callback(rip);
        rbp = unsafe { *(rbp as *const usize) };
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 8;
