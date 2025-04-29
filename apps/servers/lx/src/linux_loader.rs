use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
    TooShortImage,
    InvalidMagic,
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

pub fn load_riscv_image(hvspace: &HvSpace, image: &[u8]) -> Result<GPAddr, Error> {
    if image.len() < size_of::<RiscvImageHeader>() {
        return Err(Error::TooShortImage);
    }

    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };
    info!("header: {:x?}", header);
    let magic = u64::from_le(header.magic);
    let magic2 = u32::from_le(header.magic2);
    if magic != 0x5643534952 || magic2 != 0x5435352 {
        return Err(Error::InvalidMagic);
    }

    let text_offset = u64::from_le(header.text_offset);
    info!("text_offset: {:x}", text_offset);

    Ok(todo!())

    // let folio = Folio::alloc(4096).map_err(Error::AllocFolio)?;
    // let vaddr = VmSpace::map_anywhere_current(
    //     &folio,
    //     GUEST_MEMORY_SIZE,
    //     PageProtect::READABLE | PageProtect::WRITEABLE,
    // )
    // .map_err(Error::VmSpaceMap)?;

    // let guest_memory: &mut [u8] =
    //     unsafe { core::slice::from_raw_parts_mut(vaddr.as_mut_ptr(), GUEST_MEMORY_SIZE) };

    // // Copy the boot code to the guest memory.
    // unsafe {
    //     core::ptr::copy_nonoverlapping(
    //         BOOT_CODE.as_ptr(),
    //         guest_memory.as_mut_ptr(),
    //         BOOT_CODE.len(),
    //     );
    // };

    // hvspace
    //     .map(
    //         GPAddr::new(GUEST_ENTRY),
    //         &folio,
    //         GUEST_MEMORY_SIZE,
    //         PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
    //     )
    //     .unwrap();

    // Ok(GPAddr::new())
}
