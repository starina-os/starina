#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]
#![feature(arbitrary_self_types)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(allocator_api)]
#![feature(fn_align)]
#![feature(map_try_insert)]
#![allow(unused)]

extern crate alloc;

use core::mem::MaybeUninit;

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
mod hvspace;
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
mod vcpu;
mod vmspace;

const EARLY_RAM_SIZE: usize = 256 * 1024;
static EARLY_RAM: [MaybeUninit<u8>; EARLY_RAM_SIZE] = [MaybeUninit::uninit(); EARLY_RAM_SIZE];

pub struct BootInfo {
    dtb: *const u8,
    cpu_id: CpuId,
}

pub fn boot(bootinfo: BootInfo) -> ! {
    info!("Booting Starina...");

    GLOBAL_ALLOCATOR.add_region(EARLY_RAM.as_ptr() as *mut _, EARLY_RAM.len());

    let device_tree = device_tree::parse(bootinfo.dtb).expect("failed to parse device tree");
    cpuvar::percpu_init(bootinfo.cpu_id);
    arch::percpu_init();
    // arch::hypervisor_test();
    startup::load_inkernel_apps(device_tree);
    thread::switch_thread();
}
