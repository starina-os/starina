#![allow(unused)]

use core::marker::PhantomData;

use starina::error::ErrorCode;
use starina_types::address::PAddr;
use starina_types::address::VAddr;

use crate::arch::paddr2vaddr;
use crate::folio::Folio;

#[allow(unused)]
pub trait Endianness {
    fn to_host_u16(&self, n: u16) -> u16;
    fn to_host_u32(&self, n: u32) -> u32;
    fn to_host_u64(&self, n: u64) -> u64;
    fn from_host_u16(n: u16) -> u16;
    fn from_host_u32(n: u32) -> u32;
    fn from_host_u64(n: u64) -> u64;
}

pub struct LittleEndian;

impl Endianness for LittleEndian {
    fn to_host_u16(&self, n: u16) -> u16 {
        u16::from_le(n)
    }
    fn to_host_u32(&self, n: u32) -> u32 {
        u32::from_le(n)
    }
    fn to_host_u64(&self, n: u64) -> u64 {
        u64::from_le(n)
    }
    fn from_host_u16(n: u16) -> u16 {
        u16::to_le(n)
    }
    fn from_host_u32(n: u32) -> u32 {
        u32::to_le(n)
    }
    fn from_host_u64(n: u64) -> u64 {
        u64::to_le(n)
    }
}

pub struct BigEndian;

impl Endianness for BigEndian {
    fn to_host_u16(&self, n: u16) -> u16 {
        u16::from_be(n)
    }
    fn to_host_u32(&self, n: u32) -> u32 {
        u32::from_be(n)
    }
    fn to_host_u64(&self, n: u64) -> u64 {
        u64::from_be(n)
    }
    fn from_host_u16(n: u16) -> u16 {
        u16::to_be(n)
    }
    fn from_host_u32(n: u32) -> u32 {
        u32::to_be(n)
    }
    fn from_host_u64(n: u64) -> u64 {
        u64::to_be(n)
    }
}

pub trait Access {}
pub struct ReadOnly;
pub struct WriteOnly;
pub struct ReadWrite;

impl Access for ReadOnly {}
impl Access for WriteOnly {}
impl Access for ReadWrite {}

pub struct MmioReg<E: Endianness, A: Access, T: Copy> {
    offset: usize,
    _pd1: PhantomData<E>,
    _pd2: PhantomData<A>,
    _pd3: PhantomData<T>,
}

impl<E: Endianness, A: Access, T: Copy> MmioReg<E, A, T> {
    pub const fn new(offset: usize) -> MmioReg<E, A, T> {
        MmioReg {
            offset,
            _pd1: PhantomData,
            _pd2: PhantomData,
            _pd3: PhantomData,
        }
    }

    /// Reads a value from the MMIO region.
    ///
    /// # Why is `&mut F` required?
    ///
    /// This is to ensure that the caller has exclusive access to the MMIO
    /// region. This is important because reads from MMIO may have side effects
    /// (e.g. clearing an interrupt) and concurrent access to the same MMIO
    /// region might lead to unexpected behavior.
    ///
    /// TODO: What about memory ordering?
    fn do_read(&self, folio: &mut MmioFolio) -> T {
        self.do_read_with_offset(folio, 0)
    }

    pub fn do_read_with_offset(&self, folio: &mut MmioFolio, index: usize) -> T {
        let byte_offset = self.offset + index * size_of::<T>();
        assert!(byte_offset + size_of::<T>() <= folio.folio.len());

        let vaddr = folio.vaddr.as_usize() + byte_offset;

        unsafe { core::ptr::read_volatile(vaddr as *const T) }
    }

    fn do_write_with_offset(&self, folio: &mut MmioFolio, index: usize, value: T) {
        let byte_offset = self.offset + index * size_of::<T>();
        assert!(byte_offset + size_of::<T>() <= folio.folio.len());

        let vaddr = folio.vaddr.as_usize() + byte_offset;
        unsafe { core::ptr::write_volatile(vaddr as *mut T, value) };
    }

    fn do_write(&self, folio: &mut MmioFolio, value: T) {
        self.do_write_with_offset(folio, 0, value);
    }
}

impl<E: Endianness, T: Copy> MmioReg<E, ReadOnly, T> {
    pub fn read(&self, folio: &mut MmioFolio) -> T {
        self.do_read(folio)
    }

    pub fn read_with_offset(&self, folio: &mut MmioFolio, offset: usize) -> T {
        self.do_read_with_offset(folio, offset)
    }
}

impl<E: Endianness, T: Copy> MmioReg<E, WriteOnly, T> {
    pub fn write(&self, folio: &mut MmioFolio, value: T) {
        self.do_write(folio, value)
    }
}

impl<E: Endianness, T: Copy> MmioReg<E, ReadWrite, T> {
    pub fn read(&self, folio: &mut MmioFolio) -> T {
        self.do_read(folio)
    }

    pub fn read_with_offset(&self, folio: &mut MmioFolio, offset: usize) -> T {
        self.do_read_with_offset(folio, offset)
    }

    pub fn write(&self, folio: &mut MmioFolio, value: T) {
        self.do_write(folio, value)
    }

    pub fn write_with_offset(&self, folio: &mut MmioFolio, offset: usize, value: T) {
        self.do_write_with_offset(folio, offset, value)
    }
}

pub struct MmioFolio {
    folio: Folio,
    vaddr: VAddr,
}

impl MmioFolio {
    /// # Note
    ///
    /// `folio` must be already mapped to the kernel address space.
    pub fn from_folio(folio: Folio) -> Result<MmioFolio, ErrorCode> {
        let vaddr = paddr2vaddr(folio.paddr())?;
        Ok(MmioFolio { folio, vaddr })
    }

    pub fn paddr(&self) -> PAddr {
        self.folio.paddr()
    }
}
