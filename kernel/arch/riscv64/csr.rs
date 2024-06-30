use core::arch::asm;

#[repr(usize)]
pub enum TrapMode {
    Direct = 0,
}

pub unsafe fn write_stvec(addr: usize, mode: TrapMode) {
    assert!(addr & 0b11 == 0, "addr is not aligned");
    asm!("csrw stvec, {}", in(reg) (addr | mode as usize));
}
