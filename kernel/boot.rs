use ftl_inlinedvec::InlinedVec;
use ftl_utils::byte_size::ByteSize;

use crate::arch;
use crate::cpuvar;
use crate::cpuvar::CpuId;
use crate::device_tree::DeviceTree;
use crate::memory;
use crate::process;
use crate::startup;

include!(concat!(env!("OUT_DIR"), "/kernel_autogen.rs"));

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
    pub free_mems: InlinedVec<FreeMem, 8>,
    pub dtb_addr: *const u8,
}

/// The entry point of the kernel.
pub fn boot(cpu_id: CpuId, bootinfo: BootInfo) -> ! {
    info!("FTL - Faster Than \"L\"");

    memory::init(&bootinfo);

    let device_tree = DeviceTree::parse(bootinfo.dtb_addr);
    process::init();
    cpuvar::percpu_init(cpu_id);
    arch::init(cpu_id, &device_tree);

    startup::load_startup_apps(&STARTUP_APPS, &device_tree);
    arch::return_to_user();
}
