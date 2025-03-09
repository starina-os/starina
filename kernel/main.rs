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
use channel::Channel;
use cpuvar::CpuId;
use handle::Handle;
use poll::Poll;
use starina::app::App;
use starina::handle::HandleRights;
use starina::syscall::poll_wait;

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

    {
        use process::KERNEL_PROCESS;
        use scheduler::GLOBAL_SCHEDULER;
        let (ch1, ch2) = Channel::new().unwrap();
        let ch1_handle = KERNEL_PROCESS
            .handles()
            .lock()
            .insert(Handle::new(ch1, HandleRights::READ | HandleRights::WRITE))
            .unwrap();
        let ch2_handle = KERNEL_PROCESS
            .handles()
            .lock()
            .insert(Handle::new(ch2, HandleRights::READ | HandleRights::WRITE))
            .unwrap();

        GLOBAL_SCHEDULER.push(
            thread::Thread::new_inkernel(ktest::app_main as usize, ch1_handle.as_raw() as usize)
                .unwrap(),
        );
        GLOBAL_SCHEDULER.push(
            thread::Thread::new_inkernel(ktest::app_main as usize, ch2_handle.as_raw() as usize)
                .unwrap(),
        );
    }

    thread::switch_thread();
}

#[cfg(not(target_os = "none"))]
fn main() {
    unreachable!("added to make rust-analyzer happy");
}
