#![no_std]
#![no_main]

use core::arch::global_asm;

use arrayvec::ArrayVec;
use ftl_kernel::boot::BootInfo;
use ftl_kernel::boot::FreeMem;
use ftl_kernel::cpuvar::CpuId;
use ftl_utils::byte_size::ByteSize;

global_asm!(include_str!("boot.S"));

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[no_mangle]
unsafe extern "C" fn riscv64_boot(hartid: u64, dtb_addr: u64) -> ! {
    let bss_start = &__bss as *const _ as usize;
    let bss_end = &__bss_end as *const _ as usize;
    let free_ram = &__free_ram as *const _ as usize;
    let free_ram_end = &__free_ram_end as *const _ as usize;

    // Clear bss section.
    core::ptr::write_bytes(bss_start as *mut u8, 0, bss_end - bss_start);

    let mut free_mems = ArrayVec::<FreeMem, 8>::new();
    free_mems.push(FreeMem {
        start: free_ram,
        size: ByteSize(free_ram_end - free_ram),
    });

    ftl_kernel::boot::boot(
        CpuId::new(hartid.try_into().expect("too big hartid")),
        BootInfo {
            free_mems,
            dtb_addr: dtb_addr as *const u8,
        },
    );
}
