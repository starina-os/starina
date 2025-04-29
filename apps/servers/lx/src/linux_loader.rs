use starina::address::GPAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    VmSpaceMap(ErrorCode),
}

pub fn load_riscv_image(hvspace: &mut HvSpace, elf: &[u8]) -> Result<GPAddr, Error> {
    let folio = Folio::alloc(4096).map_err(Error::AllocFolio)?;
    let vaddr = VmSpace::map_anywhere_current(
        &folio,
        GUEST_MEMORY_SIZE,
        PageProtect::READABLE | PageProtect::WRITEABLE,
    )
    .map_err(Error::VmSpaceMap)?;

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
