//! The kernel entry point.
use ftl_inlinedvec::InlinedString;
use ftl_inlinedvec::InlinedVec;
use ftl_utils::byte_size::ByteSize;

use crate::arch;
use crate::cpuvar;
use crate::cpuvar::CpuId;
use crate::device_tree::DeviceTree;
use crate::memory;
use crate::process;
use crate::startup;
use crate::thread::Thread;

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
    pub cmdline: Option<InlinedString<126>>,
    pub free_mems: InlinedVec<FreeMem, 8>,
    pub dtb_addr: Option<*const u8>,
}

pub const USERMODE_ENABLED: bool = option_env!("USERMODE").is_some();

/// The entry point of the kernel.
pub fn boot(cpu_id: CpuId, bootinfo: BootInfo) -> ! {
    arch::early_init(cpu_id);

    println!();
    info!("FTL - Faster Than \"L\"");
    if USERMODE_ENABLED {
        info!("Usermode isolation enabled");
    } else {
        info!("Usermode isolation disabled - set USERMODE=1 to enable");
    }

    if let Some(cmdline) = &bootinfo.cmdline {
        trace!("cmdline: {}", cmdline);
    }

    // Memory subystem should be initialized first to enable dynamic memory
    // allocation.
    memory::init(&bootinfo);

    let device_tree: Option<DeviceTree> = bootinfo.dtb_addr.map(DeviceTree::parse);
    process::init();
    cpuvar::percpu_init(cpu_id);
    arch::init(cpu_id, device_tree.as_ref());

    trace!("loading startup apps...");
    startup::load_startup_apps(device_tree.as_ref(), &bootinfo);

    trace!("starting the apps...");
    Thread::switch();
}
