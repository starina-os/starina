use core::arch::asm;

#[repr(usize)]
pub enum StvecMode {
    Direct = 0,
}

pub unsafe fn write_stvec(addr: usize, mode: StvecMode) {
    assert!(addr & 0b11 == 0, "addr is not aligned");
    unsafe {
        asm!("csrw stvec, {}", in(reg) (addr | mode as usize));
    }
}

pub unsafe fn read_time() -> u64 {
    let time: u64;
    unsafe {
        asm!("csrr {}, time", out(reg) time);
    }
    time
}

pub unsafe fn write_stimecmp(value: u64) {
    unsafe {
        asm!("csrw stimecmp, {}", in(reg) value);
    }
}
