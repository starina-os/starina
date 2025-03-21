use core::arch::asm;
use core::arch::global_asm;

use arrayvec::ArrayVec;
use csr::StvecMode;
use csr::write_stvec;
use plic::use_plic;
use starina::address::DAddr;
use starina::address::PAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use transition::switch_to_kernel;

use crate::BootInfo;
use crate::FreeRam;
use crate::cpuvar::CpuId;

global_asm!(include_str!("boot.S"));

mod cpuvar;
mod csr;
mod idle;
mod interrupt;
mod plic;
mod sbi;
mod thread;
mod transition;
mod vmspace;

pub use cpuvar::CpuVar;
pub use cpuvar::get_cpuvar;
pub use cpuvar::set_cpuvar;
pub use idle::idle;
pub use interrupt::INTERRUPT_CONTROLLER;
pub use thread::Thread;
pub use thread::enter_kernelland;
pub use thread::enter_userland;
pub use vmspace::VmSpace;

pub const PAGE_SIZE: usize = 4096;
pub const NUM_CPUS_MAX: usize = 4;

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn kernel_scope<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    unsafe {
        asm!("csrrw tp, sscratch, tp");
        let ret = f();
        asm!("csrrw tp, sscratch, tp");
        ret
    }
}

pub fn map_daddr(paddr: PAddr) -> Result<DAddr, ErrorCode> {
    // We don't have IOMMU. Device will see the same address as the kernel.
    Ok(DAddr::new(paddr.as_usize()))
}

pub fn unmap_daddr(daddr: DAddr) -> Result<(), ErrorCode> {
    // We don't do anything in map_daddr. Nothing to unmap.
    Ok(())
}

pub fn vaddr2paddr(vaddr: VAddr) -> Result<PAddr, ErrorCode> {
    // Identical mapping.
    // FIXME:
    Ok(PAddr::new(vaddr.as_usize()))
}

pub fn paddr2vaddr(paddr: PAddr) -> Result<VAddr, ErrorCode> {
    // Identical mapping.
    // FIXME:
    Ok(VAddr::new(paddr.as_usize()))
}

pub fn console_write(bytes: &[u8]) {
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn riscv64_boot(hartid: u64, dtb: *const u8) -> ! {
    let bss_start = &raw const __bss as usize;
    let bss_end = &raw const __bss_end as usize;
    let free_ram = &raw const __free_ram as usize;
    let free_ram_end = &raw const __free_ram_end as usize;

    // Clear bss.
    unsafe {
        core::ptr::write_bytes(bss_start as *mut u8, 0, bss_end - bss_start);
    }

    let cpu_id = CpuId::new(hartid.try_into().unwrap());

    let mut free_rams = ArrayVec::new();
    free_rams.push(FreeRam {
        addr: free_ram as *mut u8,
        size: free_ram_end - free_ram,
    });

    crate::boot(BootInfo {
        cpu_id,
        free_rams,
        dtb,
    });
}

pub fn percpu_init() {
    unsafe {
        asm!("csrw sscratch, tp");
    }

    unsafe {
        write_stvec(switch_to_kernel as *const () as usize, StvecMode::Direct);

        let mut sie: u64;
        asm!("csrr {}, sie", out(reg) sie);
        sie |= 1 << 1; // SSIE: supervisor-level software interrupts
        sie |= 1 << 5; // STIE: supervisor-level timer interrupts
        sie |= 1 << 9; // SEIE: supervisor-level external interrupts
        asm!("csrw sie, {}", in(reg) sie);
    }

    use_plic(|plic| {
        plic.init_per_cpu(get_cpuvar().cpu_id);
    });
}
