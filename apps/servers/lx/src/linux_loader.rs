use starina::address::GPAddr;
use starina::error::ErrorCode;

use crate::guest_memory::Ram;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
    TooShortImage,
    InvalidMagic,
    InvalidImageSize,
}

/// <https://www.kernel.org/doc/html/v5.5/riscv/boot-image-header.html>
#[derive(Debug)]
#[repr(C)]
pub struct RiscvImageHeader {
    code0: u32,
    code1: u32,
    text_offset: u64,
    image_size: u64,
    flags: u64,
    version: u32,
    reserved1: u32,
    reserved2: u64,
    /// `"RISCV"` in little-endian.
    magic: u64,
    /// `"RSC\x05"` in little-endian.
    magic2: u32,
    /// Where's the `reserved3`? I think it's a typo in the doc.
    reserved3: u32,
}

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub fn load_riscv_image(
    ram: &mut Ram,
    guest_memory_base: GPAddr,
    image: &[u8],
) -> Result<GPAddr, Error> {
    if image.len() < size_of::<RiscvImageHeader>() {
        return Err(Error::TooShortImage);
    }

    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };

    // `magic` will be deprecated as per the doc. Check `magic2` only.
    let magic2 = u32::from_le(header.magic2);
    if magic2 != 0x5435352 {
        return Err(Error::InvalidMagic);
    }

    let image_size: usize = u64::from_le(header.image_size).try_into().unwrap();
    if image_size > image.len() {
        return Err(Error::InvalidImageSize);
    }

    // Load the image to "text_offset bytes from a 2MB aligned base address
    // anywhere in usable system RAM"
    // https://www.kernel.org/doc/Documentation/arm64/booting.txt
    let base = guest_memory_base.as_usize();
    let text_offset: usize = u64::from_le(header.text_offset).try_into().unwrap();
    let offset = align_up(base, 2 * 1024 * 1024) + text_offset - base;

    ram.bytes_mut()[offset..offset + image_size].copy_from_slice(&image[..image_size]);
    let image_gpaddr = guest_memory_base.checked_add(offset).unwrap();
    Ok(image_gpaddr)
}
