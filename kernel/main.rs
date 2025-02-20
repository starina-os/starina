#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(test, feature(test))]
#![no_main]

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use starina::address::PAddr;

extern crate alloc;

#[macro_use]
mod print;

mod allocator;
mod arch;
mod panic;
mod spinlock;

pub struct FreeRam {
    addr: *mut u8,
    size: usize,
}

pub struct BootInfo {
    free_rams: ArrayVec<FreeRam, 8>,
}

pub fn boot(bootinfo: BootInfo) -> ! {
    println!("\nBooting Starina...");
    for free_ram in bootinfo.free_rams {
        println!(
            "Free RAM: {:x} ({} MB)",
            free_ram.addr as usize,
            free_ram.size / 1024 / 1024
        );
        GLOBAL_ALLOCATOR.add_region(free_ram.addr, free_ram.size);
    }

    {
        use alloc::vec::Vec;

        let mut v = Vec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        println!("v: {:?}", v);
    }

    arch::halt();
}
