use core::arch::asm;
use core::arch::global_asm;

use starina::address::DAddr;
use starina::address::PAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;

use crate::BootInfo;
use crate::FreeRam;
use crate::cpuvar::CpuId;

mod boot;
mod cpuvar;
mod csr;
mod idle;
mod interrupt;
mod plic;
mod sbi;
mod serial;
mod thread;
mod transition;
mod vmspace;

pub use boot::percpu_init;
pub use cpuvar::CpuVar;
pub use cpuvar::get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use idle::halt;
pub use idle::idle;
pub use interrupt::INTERRUPT_CONTROLLER;
pub use serial::console_write;
pub use thread::Thread;
pub use thread::enter_kernelland;
pub use thread::enter_userland;
pub use transition::kernel_scope;
pub use vmspace::PAGE_SIZE;
pub use vmspace::VmSpace;
pub use vmspace::map_daddr;
pub use vmspace::paddr2vaddr;
pub use vmspace::unmap_daddr;
pub use vmspace::vaddr2paddr;
