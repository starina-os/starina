use ftl_inlinedvec::InlinedVec;
use ftl_types::spec::Spec;
use ftl_types::spec::SpecFile;
use ftl_utils::byte_size::ByteSize;

use crate::arch;
use crate::autopilot::Autopilot;
use crate::bootfs::Bootfs;
use crate::cpuvar;
use crate::cpuvar::CpuId;
use crate::memory;
use crate::process;

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
    println!("\nFTL - Faster Than \"L\"\n");

    memory::init(&bootinfo);
    arch::init();
    process::init();
    cpuvar::percpu_init(cpu_id);

    let bootfs = Bootfs::load();

    for file in bootfs.files() {
        println!("bootfs: file: {}", file.name);
    }

    let spec_file: SpecFile = serde_json::from_slice(include_bytes!("../boot.spec.json"))
        .expect("failed to parse boot spec");
    let boot_spec = match spec_file.spec {
        Spec::Boot(boot_spec) => boot_spec,
        _ => panic!("unexpected boot spec"),
    };

    let mut autopilot = Autopilot::new();
    autopilot.boot(&bootfs, &boot_spec);

    arch::yield_cpu();

    panic!("halt");
}
