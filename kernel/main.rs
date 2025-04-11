#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(allocator_api)]
#![feature(fn_align)]
#![feature(map_try_insert)]
#![allow(unused)]

extern crate alloc;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use channel::Channel;
use cpuvar::CpuId;
use handle::Handle;
use starina::device_tree::DeviceTree;
use starina_types::handle::HandleRights;

#[macro_use]
mod print;

mod allocator;
mod arch;
mod channel;
mod cpuvar;
mod device_tree;
mod folio;
mod handle;
mod interrupt;
mod iobus;
mod isolation;
mod panic;
mod poll;
mod process;
mod refcount;
mod scheduler;
mod spinlock;
mod startup;
mod syscall;
mod thread;
mod utils;
mod vmspace;

pub struct FreeRam {
    addr: *mut u8,
    size: usize,
}

pub struct BootInfo {
    dtb: *const u8,
    cpu_id: CpuId,
    free_rams: ArrayVec<FreeRam, 8>,
}

pub fn boot(bootinfo: BootInfo) -> ! {
    info!("Booting Starina...");
    for free_ram in bootinfo.free_rams {
        debug!(
            "Free RAM: {:x} ({} MB)",
            free_ram.addr as usize,
            free_ram.size / 1024 / 1024
        );
        GLOBAL_ALLOCATOR.add_region(free_ram.addr, free_ram.size);
    }

    let device_tree = device_tree::parse(bootinfo.dtb).expect("failed to parse device tree");

    cpuvar::percpu_init(bootinfo.cpu_id);
    arch::percpu_init();
    startup::load_inkernel_apps(device_tree);

    starina_quickjs::test();

    thread::switch_thread();
}
