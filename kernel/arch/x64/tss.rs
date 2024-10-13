use core::arch::asm;
use core::mem::offset_of;

use super::gdt::KERNEL_TSS;
use super::paddr2vaddr;
use crate::folio::Folio;

#[repr(C, packed)]
pub struct Tss {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_offset: u16,
    iomap: [u8; 8191],
    iomap_last_byte: u8,
}

pub static TSS: spin::Lazy<Tss> = spin::Lazy::new(|| {
    const INTR_STACK_SIZE: usize = 128 * 1024;
    let folio = Folio::alloc(INTR_STACK_SIZE).unwrap();
    let folio_vaddr = paddr2vaddr(folio.paddr()).unwrap().as_usize();
    let rsp0 = (folio_vaddr + INTR_STACK_SIZE) as u64;
    Tss {
        reserved0: 0,
        rsp0,
        rsp1: 0,
        rsp2: 0,
        reserved1: 0,
        ist: [0; 7],
        reserved2: 0,
        reserved3: 0,
        iomap_offset: offset_of!(Tss, iomap) as u16,
        iomap: [0; 8191],
        iomap_last_byte: 0xff, // TODO: do we need to fill this?
    }
});

pub fn init() {
    unsafe {
        asm!("ltr ax", in("ax") KERNEL_TSS);
    }
}
