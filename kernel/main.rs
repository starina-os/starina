#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate starina;

extern crate alloc;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use cpuvar::CpuId;
use starina::app::App;

mod scheduler;
mod cpuvar;
mod allocator;
mod arch;
mod panic;
mod syscall;
mod spinlock;
mod thread;
mod refcount;

pub struct FreeRam {
    addr: *mut u8,
    size: usize,
}

pub struct BootInfo {
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

    cpuvar::percpu_init(bootinfo.cpu_id);

    let mut apps: Vec<Box<dyn App>> = Vec::new();
    apps.push(Box::new(ktest::Main::init()));

    thread::switch_thread();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
