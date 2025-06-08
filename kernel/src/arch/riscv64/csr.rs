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

