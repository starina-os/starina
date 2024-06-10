use arrayvec::ArrayVec;
use ftl_utils::byte_size::ByteSize;

use crate::arch;
use crate::cpuvar;
use crate::cpuvar::CpuId;
use crate::memory;
use crate::thread::Thread;

/// A free region of memory available for software.
#[derive(Debug)]
pub struct FreeMem {
    /// The start address of the region.
    pub start: usize,
    /// The size of the region.
    pub size: ByteSize,
}

/// The boot information passed from the bootloader.
#[derive(Debug)]
pub struct BootInfo {
    pub free_mems: ArrayVec<FreeMem, 8>,
    pub dtb_addr: *const u8,
}

#[no_mangle]
fn thread_entry(thread_id: usize) {
    let ch = char::from_u32(('A' as usize + thread_id) as u32).unwrap();
    for i in 0.. {
        println!("{}: {}", ch, i);
        for _ in 0..0x100000 {}
        arch::yield_cpu();
    }
}

/// The entry point of the kernel.
pub fn boot(cpu_id: CpuId, bootinfo: BootInfo) -> ! {
    println!("\nFTL - Faster Than \"L\"\n");

    memory::init(&bootinfo);
    cpuvar::percpu_init(cpu_id);

    let mut v = alloc::vec::Vec::new();
    v.push(alloc::string::String::from("Hello, "));
    v.push(alloc::string::String::from("world!"));
    println!("alloc test: {:?}", v);

    println!("cpuvar test: CPU {}", arch::cpuvar().cpu_id);

    oops!("backtrace test");

    Thread::spawn_kernel(thread_entry, 0);
    Thread::spawn_kernel(thread_entry, 1);
    Thread::spawn_kernel(thread_entry, 2);
    Thread::spawn_kernel(thread_entry, 3);
    arch::yield_cpu();

    println!("kernel is ready!");
    arch::halt();
}
