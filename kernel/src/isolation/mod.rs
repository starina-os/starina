use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::slice;

use starina_types::error::ErrorCode;

mod inkernel;

pub use inkernel::INKERNEL_ISOLATION;

/// A pointer in an isolation space.
///
/// This is an opaque value and depends on the isolation implementation. For example,
/// it is a raw kernel pointer in the in-kernel isolation, a user pointer in the
/// user-space isolation, or a memory offset in WebAssembly isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolationPtr(usize);

impl IsolationPtr {
    pub const fn new(ptr: usize) -> Self {
        Self(ptr)
    }
}

fn checked_ptr(
    base_ptr: usize,
    max_len: usize,
    offset: usize,
    len: usize,
) -> Result<IsolationPtr, ErrorCode> {
    // Check overflows.
    let ptr = base_ptr.checked_add(offset).ok_or(ErrorCode::TooLarge)?;
    let end_offset = offset.checked_add(len).ok_or(ErrorCode::TooLarge)?;

    // Check if it's within the slice bounds.
    if end_offset > max_len {
        return Err(ErrorCode::TooLarge);
    }

    Ok(IsolationPtr::new(ptr))
}

/// A slice in an isolation space.
pub struct IsolationSlice {
    ptr: IsolationPtr,
    len: usize,
}

impl IsolationSlice {
    pub const fn new(ptr: IsolationPtr, len: usize) -> Self {
        Self { ptr, len }
    }

    pub fn read<T: Copy>(&self, isolation: &dyn Isolation, offset: usize) -> Result<T, ErrorCode> {
        let checked_ptr = checked_ptr(self.ptr.0, self.len, offset, size_of::<T>())?;

        let mut buf = MaybeUninit::uninit();
        let buf_ptr = buf.as_mut_ptr() as *mut u8;
        let buf_slice = unsafe { slice::from_raw_parts_mut(buf_ptr, size_of::<T>()) };

        isolation.read_bytes(checked_ptr, buf_slice)?;
        Ok(unsafe { buf.assume_init() })
    }

    pub fn read_to_vec(
        &self,
        isolation: &dyn Isolation,
        offset: usize,
        len: usize,
    ) -> Result<Vec<u8>, ErrorCode> {
        let checked_ptr = checked_ptr(self.ptr.0, self.len, offset, len)?;

        let mut buf = Vec::new();
        buf.resize(len, 0);
        isolation.read_bytes(checked_ptr, &mut buf)?;
        Ok(buf)
    }
}

/// A mutable slice in an isolation space.
pub struct IsolationSliceMut {
    slice: IsolationSlice,
}

impl IsolationSliceMut {
    pub const fn new(ptr: IsolationPtr, len: usize) -> Self {
        Self {
            slice: IsolationSlice::new(ptr, len),
        }
    }

    pub fn write<T: Copy>(
        &self,
        isolation: &dyn Isolation,
        offset: usize,
        value: T,
    ) -> Result<(), ErrorCode> {
        let checked_ptr = checked_ptr(self.slice.ptr.0, self.slice.len, offset, size_of::<T>())?;
        let value_ptr = &value as *const T as *mut u8;
        let value_bytes = unsafe { slice::from_raw_parts_mut(value_ptr, size_of::<T>()) };
        isolation.write_bytes(checked_ptr, value_bytes)?;
        Ok(())
    }

    pub fn write_bytes(
        &self,
        isolation: &dyn Isolation,
        offset: usize,
        slice: &[u8],
    ) -> Result<(), ErrorCode> {
        let checked_ptr = checked_ptr(self.slice.ptr.0, self.slice.len, offset, slice.len())?;
        isolation.write_bytes(checked_ptr, slice)
    }
}

impl Deref for IsolationSliceMut {
    type Target = IsolationSlice;

    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}

/// Memory isolation, such as in-kernel isolation or user-space isolation.
///
/// This trait defines how to access memory in an isolation space.
pub trait Isolation: Send + Sync {
    fn read_bytes(&self, ptr: IsolationPtr, dst: &mut [u8]) -> Result<(), ErrorCode>;
    fn write_bytes(&self, ptr: IsolationPtr, src: &[u8]) -> Result<(), ErrorCode>;
}
