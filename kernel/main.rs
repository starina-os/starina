#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use starina::app::App;
use starina::{debug, info};

extern crate alloc;

mod allocator;
mod arch;
mod panic;
mod syscall;
mod spinlock;

pub struct FreeRam {
    addr: *mut u8,
    size: usize,
}

pub struct BootInfo {
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

    let mut apps: Vec<Box<dyn App>> = Vec::new();
    apps.push(Box::new(ktest::Main::init()));

    arch::halt();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
