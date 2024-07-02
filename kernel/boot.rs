use alloc::string::String;
use alloc::vec;

use ftl_inlinedvec::InlinedVec;
use ftl_types::spec::AppSpec;
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

    fn load_app_spec(spec: &[u8], elf_file: &'static [u8]) -> (String, AppSpec, &'static [u8]) {
        let spec_file: SpecFile = serde_json::from_slice(spec).expect("failed to parse app spec");
        let Spec::App(app_spec) = spec_file.spec;
        (spec_file.name, app_spec, elf_file)
    }

    let mut autopilot = Autopilot::new();
    autopilot
        .start_apps(vec![
            load_app_spec(
                include_bytes!("../apps/ping/app.spec.json"),
                &bootfs.find_by_name("apps/ping.elf").unwrap().data,
            ),
            load_app_spec(
                include_bytes!("../apps/pong/app.spec.json"),
                &bootfs.find_by_name("apps/pong.elf").unwrap().data,
            ),
            // load_app_spec(
            //     include_bytes!("../apps/virtio_blk/app.spec.json"),
            //     &bootfs.find_by_name("apps/virtio_blk.elf").unwrap().data,
            // ),
        ])
        .expect("failed to start apps");

    arch::yield_cpu();

    panic!("halt");
}
