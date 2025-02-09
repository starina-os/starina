#![no_std]
#![no_main]
#![cfg(target_arch = "riscv64")]

use core::arch::global_asm;

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

    kernel::boot();
}
