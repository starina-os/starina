use core::arch::asm;
use core::arch::global_asm;

use arrayvec::ArrayVec;

use crate::BootInfo;
use crate::FreeRam;

global_asm!(include_str!("boot.S"));

mod sbi;

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

pub fn console_write(bytes: &[u8]) {
    for byte in bytes {
        sbi::console_putchar(*byte);
    }
}

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn riscv64_boot(_hartid: u64, _dtb_addr: u64) -> ! {
    let bss_start = &raw const __bss as usize;
    let bss_end = &raw const __bss_end as usize;
    let free_ram = &raw const __free_ram as usize;
    let free_ram_end = &raw const __free_ram_end as usize;

    // Clear bss.
    unsafe {
        core::ptr::write_bytes(bss_start as *mut u8, 0, bss_end - bss_start);
    }

    let mut free_rams = ArrayVec::new();
    free_rams.push(FreeRam {
        addr: free_ram as *mut u8,
        size: free_ram_end - free_ram,
    });

    crate::boot(BootInfo { free_rams });
}
