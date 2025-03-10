#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(allocator_api)]
#![allow(unused)]

#[macro_use]
extern crate starina;

extern crate alloc;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use cpuvar::CpuId;
use starina::app::App;

mod allocator;
mod arch;
mod channel;
mod cpuvar;
mod handle;
mod isolation;
mod panic;
mod poll;
mod process;
mod refcount;
mod scheduler;
mod spinlock;
mod syscall;
mod thread;
mod utils;

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
    arch::percpu_init();

    fn entrypoint(app: *mut ktest::Main) {
        let app = unsafe { &mut *app };
        info!("Starting app...");
        for _ in 0.. {
            app.tick();
            for _ in 0..1000000 {}
        }
    }

    // GLOBAL_SCHEDULER.push(thread::Thread::new_inkernel(
    //     entrypoint as usize,
    //     Box::leak(Box::new(ktest::Main::init())) as *const _ as usize,
    // ));
    // GLOBAL_SCHEDULER.push(thread::Thread::new_inkernel(
    //     entrypoint as usize,
    //     Box::leak(Box::new(ktest::Main::init())) as *const _ as usize,
    // ));

    thread::switch_thread();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
