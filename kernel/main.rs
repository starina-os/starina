#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]
#![feature(naked_functions)]

#[macro_use]
extern crate starina;

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use cpuvar::CpuId;
use scheduler::GLOBAL_SCHEDULER;
use starina::app::App;
use starina::syscall::thread_yield;

mod allocator;
mod arch;
mod cpuvar;
mod panic;
mod refcount;
mod scheduler;
mod spinlock;
mod syscall;
mod thread;

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

    fn entrypoint(app: *const ktest::Main) {
        let app = unsafe { &*app };
        info!("Starting app...");
        for i in 0.. {
            info!("heartbeat: {}", i);
            app.heartbeat();
            thread_yield();
            for _ in 0..1000000 {}
        }
    }

    let ktest_app: *const ktest::Main = Box::leak(Box::new(ktest::Main::init()));
    let arg = ktest_app as usize;
    let t = thread::Thread::new_inkernel(entrypoint as usize, arg);
    GLOBAL_SCHEDULER.push(t);

    thread::switch_thread();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
