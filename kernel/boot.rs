use arrayvec::ArrayVec;
use ftl_utils::byte_size::ByteSize;

use crate::arch;
use crate::cpuvar;
use crate::cpuvar::CpuId;
use crate::app_loader::AppLoader;
use crate::memory;
use crate::syscall::VSYSCALL_PAGE;

/// A free region of memory available for software.
#[derive(Debug)]
pub struct FreeMem {
    /// The start address of the region.
    pub start: usize,
    /// The size of the region.
    pub size: ByteSize,
}

/// The boot information passed from the bootloader.
#[derive(Debug)]
pub struct BootInfo {
    pub free_mems: ArrayVec<FreeMem, 8>,
    pub dtb_addr: *const u8,
}

const STARTUP_ELF: &[u8] = include_bytes!("../build/startup.elf");

/// The entry point of the kernel.
pub fn boot(cpu_id: CpuId, bootinfo: BootInfo) -> ! {
    println!("\nFTL - Faster Than \"L\"\n");

    memory::init(&bootinfo);
    cpuvar::percpu_init(cpu_id);

    AppLoader::parse(STARTUP_ELF)
        .expect("startup.elf is invalid")
        .load(&VSYSCALL_PAGE)
        .expect("failed to load startup.elf");

    arch::yield_cpu();

    println!("kernel is ready!");
    arch::halt();
}
