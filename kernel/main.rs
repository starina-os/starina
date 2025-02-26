#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(test, feature(test))]
#![feature(naked_functions)]
#![feature(arbitrary_self_types)]

#[macro_use]
extern crate starina;

extern crate alloc;

use alloc::boxed::Box;

use allocator::GLOBAL_ALLOCATOR;
use arrayvec::ArrayVec;
use channel::Channel;
use cpuvar::CpuId;
use scheduler::GLOBAL_SCHEDULER;
use starina::app::App;
use starina::message::MessageInfo;

mod allocator;
mod arch;
mod channel;
mod cpuvar;
mod handle;
mod isolation;
mod panic;
mod process;
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

    fn entrypoint(app: *mut ktest::Main) {
        let app = unsafe { &mut *app };
        info!("Starting app...");
        for _ in 0.. {
            app.tick();
            for _ in 0..1000000 {}
        }
    }

    let (ch1, ch2) = Channel::new().unwrap();
    let send_buf = b"BEEP BEEP BEEP";
    let send_heap = isolation::IsolationHeap::InKernel {
        ptr: send_buf.as_ptr() as usize,
        len: send_buf.len(),
    };
    let mut handles_heap = isolation::IsolationHeap::InKernel { ptr: 0, len: 0 };
    info!("Sending message...");
    ch1.send(
        MessageInfo::new(0, send_buf.len().try_into().unwrap(), 0),
        &send_heap,
        &handles_heap,
    )
    .unwrap();

    let mut recv_buf = [0u8; 16];
    let mut recv_heap = isolation::IsolationHeap::InKernel {
        ptr: recv_buf.as_ptr() as usize,
        len: recv_buf.len(),
    };
    ch2.recv(&mut recv_heap, &mut handles_heap).unwrap();
    // hexdump of recv_buf
    println!("received");
    for i in 0..recv_buf.len() {
        print!("{:02x} ", recv_buf[i]);
    }
    println!();

    panic!("done");

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
