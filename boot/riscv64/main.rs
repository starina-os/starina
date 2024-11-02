#![no_std]
#![no_main]
#![cfg(target_arch = "riscv64")]

use core::arch::global_asm;

use starina_inlinedvec::InlinedVec;
use starina_kernel::boot::BootInfo;
use starina_kernel::boot::FreeMem;
use starina_kernel::cpuvar::CpuId;
use starina_utils::byte_size::ByteSize;

global_asm!(include_str!("boot.S"));

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[no_mangle]
unsafe extern "C" fn riscv64_boot(hartid: u64, dtb_addr: u64) -> ! {
    let bss_start = &raw const __bss as usize;
    let bss_end = &raw const __bss_end as usize;
    let free_ram = &raw const __free_ram as usize;
    let free_ram_end = &raw const __free_ram_end as usize;

    // Clear bss section.
    core::ptr::write_bytes(bss_start as *mut u8, 0, bss_end - bss_start);

    let mut free_mems = InlinedVec::<FreeMem, 8>::new();
    free_mems
        .try_push(FreeMem {
            start: free_ram,
            size: ByteSize(free_ram_end - free_ram),
        })
        .expect("too many free mems");

    starina_kernel::boot::boot(
        CpuId::new(hartid.try_into().expect("too big hartid")),
        BootInfo {
            cmdline: None,
            free_mems,
            dtb_addr: Some(dtb_addr as *const u8),
        },
    );
}
