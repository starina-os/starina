//! Type-safe MMIO register access.
//!
//! MMIO (Memory-Mapped I/O) is a mechanism to access hardware devices using
//! memory read and write operations. Unlike normal memory accesses, in MMIO,
//! you need to carefully handle endianness and unexpected compiler/CPU
//! optimizations. This module will help you.
//!
//! # Why is `&mut MmioFolio` required in `read` methods?
//!
//! This is to ensure that the caller has exclusive access to the MMIO
//! region. This is important because reads from MMIO may have side effects
//! (e.g. clearing an interrupt) and concurrent access to the same MMIO
//! region might lead to unexpected behavior.
//!
//! # Example (Goldfish RTC driver)
//!
//! In this example, two MMIO registers are defined: `TIME_LOW_REG` and
//! `TIME_HIGH_REG` and prints the current time read from the [Goldfish RTC](https://github.com/qemu/qemu/blob/master/hw/rtc/goldfish_rtc.c)
//! device.
//!
//! To access the MMIO registers, you need to acquire and map the MMIO region
//! using `MmioFolio::create_pinned`. Then, pass the mutable reference of
//! the `MmioFolio` to the `read` method of the MMIO register:
//!
//! ```no_run
//! use starina::prelude::*;
//! use starina::folio::MmioFolio;
//! use starina::address::DAddr;
//! use starina_driver_sdk::mmio::{LittleEndian, MmioReg, ReadOnly};
//!
//! let iobus = todo!();
//! const MMIO_BASE: DAddr = DAddr::new(0x101000);
//! const MMIO_SIZE: usize = 4096;
//!
//! static TIME_LOW_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x00);
//! static TIME_HIGH_REG: MmioReg<LittleEndian, ReadOnly, u32> = MmioReg::new(0x04);
//!
//! let mut folio = MmioFolio::create_pinned(iobus, MMIO_BASE, MMIO_SIZE).unwrap();
//! let low: u32 = TIME_LOW_REG.read(&mut folio);
//! let high: u32 = TIME_HIGH_REG.read(&mut folio);
//! let now: u64 = (high as u64) << 32 | (low as u64);
//!
//! // If you want to convert the time to a human-readable format:
//! // date = chrono::DateTime::from_timestamp_nanos(now);
//! let now: i64 = now.try_into().unwrap();
//! info!("now: {now}");
//! ```
use core::marker::PhantomData;

use starina::folio::MmioFolio;

/// A trait for endianness conversion.
pub trait Endianess {
    /// Converts a device-endian `u16` to host-endian `u16`.
    fn into_host_u16(&self, n: u16) -> u16;
    /// Converts a device-endian `u32` to host-endian `u32`.
    fn into_host_u32(&self, n: u32) -> u32;
    /// Converts a device-endian `u64` to host-endian `u64`.
    fn into_host_u64(&self, n: u64) -> u64;
    /// Converts a host-endian `u16` to device-endian `u16`.
    fn from_host_u16(&self, n: u16) -> u16;
    /// Converts a host-endian `u32` to device-endian `u32`.
    fn from_host_u32(&self, n: u32) -> u32;
    /// Converts a host-endian `u64` to device-endian `u64`.
    fn from_host_u64(&self, n: u64) -> u64;
}

/// Little-endian endianness.
pub struct LittleEndian;

impl Endianess for LittleEndian {
    fn into_host_u16(&self, n: u16) -> u16 {
        u16::from_le(n)
    }
    fn into_host_u32(&self, n: u32) -> u32 {
        u32::from_le(n)
    }
    fn into_host_u64(&self, n: u64) -> u64 {
        u64::from_le(n)
    }
    fn from_host_u16(&self, n: u16) -> u16 {
        u16::to_le(n)
    }
    fn from_host_u32(&self, n: u32) -> u32 {
        u32::to_le(n)
    }
    fn from_host_u64(&self, n: u64) -> u64 {
        u64::to_le(n)
    }
}

/// Big-endian endianness.
pub struct BigEndian;

impl Endianess for BigEndian {
    fn into_host_u16(&self, n: u16) -> u16 {
        u16::from_be(n)
    }
    fn into_host_u32(&self, n: u32) -> u32 {
        u32::from_be(n)
    }
    fn into_host_u64(&self, n: u64) -> u64 {
        u64::from_be(n)
    }
    fn from_host_u16(&self, n: u16) -> u16 {
        u16::to_be(n)
    }
    fn from_host_u32(&self, n: u32) -> u32 {
        u32::to_be(n)
    }
    fn from_host_u64(&self, n: u64) -> u64 {
        u64::to_be(n)
    }
}

/// A trait for defining allowed access types.
pub trait Access {}

/// Read-only MMIO register.
pub struct ReadOnly;

/// Write-only MMIO register.
pub struct WriteOnly;

/// Read-write MMIO register.
pub struct ReadWrite;

impl Access for ReadOnly {}
impl Access for WriteOnly {}
impl Access for ReadWrite {}

/// A memory-mapped I/O register.
///
/// This struct defines a memory-mapped I/O register. It is parameterized by:
///
/// - `E`: Endianness of the register ([`LittleEndian`], [`BigEndian`]).
/// - `A`: Access type of the register ([`ReadOnly`], [`WriteOnly`], [`ReadWrite`]).
/// - `T`: Type of the register (`u8`, `u16`, `u32`, `u64`).
pub struct MmioReg<E: Endianess, A: Access, T: Copy> {
    offset: usize,
    _pd1: PhantomData<E>,
    _pd2: PhantomData<A>,
    _pd3: PhantomData<T>,
}

impl<E: Endianess, A: Access, T: Copy> MmioReg<E, A, T> {
    /// Defines a MMIO register.
    pub const fn new(offset: usize) -> MmioReg<E, A, T> {
        MmioReg {
            offset,
            _pd1: PhantomData,
            _pd2: PhantomData,
            _pd3: PhantomData,
        }
    }

    /// Reads a value from the MMIO register with an offset.
    ///
    /// This is useful when the MMIO register spans multiple words or unaligned
    /// length, such as MAC address (6 bytes).
    pub fn do_read_with_offset(&self, folio: &mut MmioFolio, offset: usize) -> T {
        let vaddr = folio.vaddr().as_usize() + self.offset + offset * size_of::<T>();
        unsafe { core::ptr::read_volatile(vaddr as *const T) }
    }

    fn do_write_with_offset(&self, folio: &mut MmioFolio, offset: usize, value: T) {
        let vaddr = folio.vaddr().as_usize() + self.offset + offset * size_of::<T>();
        unsafe { core::ptr::write_volatile(vaddr as *mut T, value) };
    }
}

impl<E: Endianess, T: Copy> MmioReg<E, ReadOnly, T> {
    /// Reads a value from the MMIO register.
    pub fn read(&self, folio: &mut MmioFolio) -> T {
        self.do_read_with_offset(folio, 0)
    }

    pub fn read_with_offset(&self, folio: &mut MmioFolio, offset: usize) -> T {
        self.do_read_with_offset(folio, offset)
    }
}

impl<E: Endianess, T: Copy> MmioReg<E, WriteOnly, T> {
    /// Writes a value to the MMIO register.
    pub fn write(&self, folio: &mut MmioFolio, value: T) {
        self.do_write_with_offset(folio, 0, value)
    }
}

impl<E: Endianess, T: Copy> MmioReg<E, ReadWrite, T> {
    /// Reads a value from the MMIO register.
    pub fn read(&self, folio: &mut MmioFolio) -> T {
        self.do_read_with_offset(folio, 0)
    }

    /// Writes a value to the MMIO register with an offset.
    pub fn read_with_offset(&self, folio: &mut MmioFolio, offset: usize) -> T {
        self.do_read_with_offset(folio, offset)
    }

    /// Writes a value to the MMIO register.
    pub fn write(&self, folio: &mut MmioFolio, value: T) {
        self.do_write_with_offset(folio, 0, value)
    }

    /// Writes a value to the MMIO register with an offset.
    pub fn write_with_offset(&self, folio: &mut MmioFolio, offset: usize, value: T) {
        self.do_write_with_offset(folio, offset, value)
    }
}
