use arrayvec::ArrayVec;
use ftl_utils::byte_size::ByteSize;

use crate::arch;

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

/// The entry point of the kernel.
pub fn boot(_cpu_id: usize, bootinfo: BootInfo) -> ! {
    println!("\nFTL - Faster Than \"L\"\n");

    for e in bootinfo.free_mems.iter() {
        println!(
            "free memory: {:#x} - {:#x} ({})",
            e.start,
            e.start + e.size.in_bytes(),
            e.size
        );
    }

    println!("kernel is ready!");
    arch::halt();
}
