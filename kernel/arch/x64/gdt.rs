use core::arch::asm;

use super::tss::Tss;
use super::tss::TSS;

#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    base: u64,
}

type GdtType = [u64; 5];
pub const KERNEL_CS: u16 = 8 * 1;
pub const KERNEL_DS: u16 = 8 * 2;
pub const KERNEL_TSS: u16 = 8 * 3;

static GDT: spin::Lazy<GdtType> = spin::Lazy::new(|| {
    let tss_addr = TSS.as_mut_ptr() as u64;
    let tss_low = 0x0000890000000000
        | size_of::<Tss>() as u64
        | ((tss_addr & 0xffff) << 16)
        | (((tss_addr >> 16) & 0xff) << 32)
        | (((tss_addr >> 24) & 0xff) << 56);
    let tss_high = tss_addr >> 32;

    [
        0x0000000000000000, // 0:  null descriptor
        0x00af9a000000ffff, // 8:  64-bit code segment (kernel)
        0x00cf92000000ffff, // 16: 64-bit data segment (kernel)
        tss_low,
        tss_high,
    ]
});

pub fn init() {
    let gdtr = Gdtr {
        limit: (core::mem::size_of::<GdtType>() - 1) as u16,
        base: GDT.as_ptr() as u64,
    };

    unsafe {
        asm!("lgdt [{}]", in(reg) &gdtr);
    }
}
