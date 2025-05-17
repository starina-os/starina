use core::mem::MaybeUninit;
use core::ops::Range;
use core::ptr;
use core::slice;

use starina::address::GPAddr;
use starina::address::VAddr;
use starina::error::ErrorCode;
use starina::folio::Folio;
use starina::hvspace::HvSpace;
use starina::prelude::*;
use starina::vmspace::PageProtect;
use starina::vmspace::VmSpace;

#[derive(Debug)]
pub enum Error {
    AllocFolio(ErrorCode),
    CreateHvSpace(ErrorCode),
    VmSpaceMap(ErrorCode),
    MapRam(ErrorCode),
    OutOfRam,
    InvalidAddress(GPAddr),
    TooLong,
}

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct GuestMemory {
    hvspace: HvSpace,
    start: GPAddr,
    end: GPAddr,
    size: usize,
    _folio: Folio,
    vaddr: VAddr,
    free_offset: usize,
}

impl GuestMemory {
    pub fn new(start: GPAddr, size: usize) -> Result<Self, Error> {
        let end = start.checked_add(size).unwrap();

        // Allocate a virtually-contiguous memory region (folio).
        let folio = Folio::alloc(size).map_err(Error::AllocFolio)?;

        // Map the folio into the current (VMM's) address space.
        let vaddr = VmSpace::map_anywhere_current(
            &folio,
            size,
            PageProtect::READABLE | PageProtect::WRITEABLE,
        )
        .map_err(Error::VmSpaceMap)?;

        // Create a guest address space and map the folio into it.
        let hvspace = HvSpace::new().map_err(Error::CreateHvSpace)?;
        hvspace
            .map(
                start,
                &folio,
                size,
                PageProtect::READABLE | PageProtect::WRITEABLE | PageProtect::EXECUTABLE,
            )
            .map_err(Error::MapRam)?;

        Ok(Self {
            hvspace,
            start,
            end,
            size,
            _folio: folio,
            vaddr,
            free_offset: 0,
        })
    }

    pub fn hvspace(&self) -> &HvSpace {
        &self.hvspace
    }

    pub fn allocate(&mut self, size: usize, align: usize) -> Result<(&mut [u8], GPAddr), Error> {
        let free_start = align_up(self.free_offset, align);
        if free_start + size > self.size {
            return Err(Error::OutOfRam);
        }

        self.free_offset = free_start + size;

        let gpaddr = self.start.checked_add(free_start).unwrap();
        let slice = &mut self.slice_mut()[free_start..free_start + size];

        trace!("RAM: allocated {} bytes at {}", size, gpaddr);
        Ok((slice, gpaddr))
    }

    fn slice(&self) -> &[u8] {
        // SAFETY: The folio is mapped to the current vmspace, and folio
        // is kept alive as long as `self` is alive.
        unsafe { slice::from_raw_parts(self.vaddr.as_ptr(), self.size) }
    }

    fn slice_mut(&self) -> &mut [u8] {
        // SAFETY: The folio is mapped to the current vmspace, and folio
        // is kept alive as long as `self` is alive.
        unsafe { slice::from_raw_parts_mut(self.vaddr.as_mut_ptr(), self.size) }
    }

    fn check_range(&self, gpaddr: GPAddr, size: usize) -> Result<Range<usize>, Error> {
        if gpaddr < self.start || gpaddr >= self.end {
            return Err(Error::InvalidAddress(gpaddr));
        }

        let Some(end) = gpaddr.checked_add(size) else {
            return Err(Error::TooLong);
        };

        if end > self.end {
            return Err(Error::TooLong);
        }

        let offset = gpaddr.as_usize() - self.start.as_usize();
        let range = offset..offset + size;
        Ok(range)
    }

    pub fn bytes_slice(&self, gpaddr: GPAddr, size: usize) -> Result<&[u8], Error> {
        let range = self.check_range(gpaddr, size)?;
        Ok(&self.slice()[range])
    }

    pub fn read_bytes(&self, gpaddr: GPAddr, dst: &mut [u8]) -> Result<(), Error> {
        let range = self.check_range(gpaddr, dst.len())?;
        let slice = &self.slice()[range];
        unsafe {
            // FIXME: Use volatile copy.
            ptr::copy_nonoverlapping(slice.as_ptr(), dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }

    pub fn write_bytes(&self, gpaddr: GPAddr, src: &[u8]) -> Result<(), Error> {
        let range = self.check_range(gpaddr, src.len())?;
        let slice = &mut self.slice_mut()[range];
        unsafe {
            // FIXME: Use volatile copy.
            ptr::copy_nonoverlapping(src.as_ptr(), slice.as_mut_ptr(), src.len());
        }
        Ok(())
    }

    pub fn read<T: Copy>(&self, gpaddr: GPAddr) -> Result<T, Error> {
        let mut buf = MaybeUninit::<T>::uninit();
        let buf_slice =
            unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, size_of::<T>()) };

        self.read_bytes(gpaddr, buf_slice)?;
        Ok(unsafe { buf.assume_init() })
    }

    pub fn write<T: Copy>(&self, gpaddr: GPAddr, value: T) -> Result<(), Error> {
        let buf_slice =
            unsafe { slice::from_raw_parts(&value as *const T as *const u8, size_of::<T>()) };

        self.write_bytes(gpaddr, buf_slice)?;
        Ok(())
    }
}
