#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate starina;

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use cpuvar::CpuId;
use scheduler::GLOBAL_SCHEDULER;
use starina::app::App;

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

    fn entrypoint(app: *const Arc<dyn App>) {
        info!("Starting app...");
        for i in 0.. {
            info!("App: {}", i);
            for _ in 0..10000000 {}
        }
    }

    let ktest_app: Arc<dyn App> = Arc::new(ktest::Main::init());
    unsafe {
        Arc::increment_strong_count(&ktest_app);
    }
    let t = thread::Thread::new_inkernel(entrypoint as usize, &ktest_app as *const _ as usize);
    GLOBAL_SCHEDULER.push(t);

    thread::switch_thread();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
