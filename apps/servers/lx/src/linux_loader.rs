use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::prelude::*;

use crate::guest_memory::Ram;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
    AllocRam(crate::guest_memory::Error),
    TooShortImage,
    InvalidMagic,
    InvalidImageSize,
}

/// <https://www.kernel.org/doc/html/v5.5/riscv/boot-image-header.html>
/// <https://www.kernel.org/doc/Documentation/arm64/booting.txt>
#[derive(Debug)]
#[repr(C)]
pub struct RiscvImageHeader {
    code0: u32,
    code1: u32,
    text_offset: u64,
    /// The size of kernel memory, beyond the kernel image itself.
    ///
    /// > At least image_size bytes from the start of the image must be free for
    /// > use by the kernel.
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

/// Load the image to "text_offset bytes from a 2MB aligned base address
/// anywhere in usable system RAM"
/// https://www.kernel.org/doc/Documentation/arm64/booting.txt
const IMAGE_ALIGN: usize = 2 * 1024 * 1024;

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub fn load_riscv_image(ram: &mut Ram, image: &[u8]) -> Result<GPAddr, Error> {
    if image.len() < size_of::<RiscvImageHeader>() {
        return Err(Error::TooShortImage);
    }

    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };

    // `magic` will be deprecated as per the doc. Check `magic2` only.
    let magic2 = u32::from_le(header.magic2);
    if magic2 != 0x5435352 {
        return Err(Error::InvalidMagic);
    }

    let kernel_size = u64::from_le(header.image_size);
    let (ram_slice, gpaddr) = ram
        .allocate(kernel_size as usize, IMAGE_ALIGN)
        .map_err(Error::AllocRam)?;

    trace!(
        "loaded image at gpaddr={}, len={}KiB",
        gpaddr,
        image.len() / 1024
    );
    ram_slice[..image.len()].copy_from_slice(&image[..image.len()]);
    Ok(gpaddr)
}
