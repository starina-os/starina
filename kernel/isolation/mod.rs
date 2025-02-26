use alloc::vec::Vec;

use starina::error::ErrorCode;

pub enum Isolation {
    InKernel,
}

pub enum IsolationHeap {
    InKernel { ptr: usize, len: usize },
}

impl IsolationHeap {
    pub fn write<T>(
        &mut self,
        isolation: &Isolation,
        offset: usize,
        value: T,
    ) -> Result<(), ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        // TODO: size check
        // TODO: wraparound check
        // TODO: alignment check
        let raw_ptr = *ptr + offset;

        unsafe {
            core::ptr::write(raw_ptr as *mut T, value);
        }

        Ok(())
    }

    pub fn write_bytes(
        &mut self,
        isolation: &Isolation,
        offset: usize,
        slice: &[u8],
    ) -> Result<(), ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        let raw_ptr = *ptr + offset;
        unsafe {
            core::ptr::copy(slice.as_ptr(), raw_ptr as *mut u8, slice.len());
        }
        Ok(())
    }

    pub fn read_to_vec(
        &self,
        isolation: &Isolation,
        offset: usize,
        len: usize,
    ) -> Result<Vec<u8>, ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        let raw_ptr = *ptr + offset;

        let slice = unsafe { core::slice::from_raw_parts(raw_ptr as *const u8, len) };
        Ok(Vec::from(slice))
    }

    pub fn read<T>(&self, isolation: &Isolation, offset: usize) -> Result<T, ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        assert!(matches!(isolation, Isolation::InKernel));

        let raw_ptr = *ptr + offset;
        unsafe { Ok(core::ptr::read(raw_ptr as *const T)) }
    }
}
