use alloc::vec::Vec;

use starina::error::ErrorCode;

pub enum Isolation {
    InKernel,
}

pub enum IsolationHeap {
    InKernel { ptr: *mut u8, len: usize },
}

impl IsolationHeap {
    pub fn write<T: Copy>(
        &mut self,
        isolation: &Isolation,
        offset: usize,
        value: T,
    ) -> Result<(), ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        // TODO: size check
        // TODO: wraparound check
        // TODO: alignment check
        unsafe {
            core::ptr::write(ptr.add(offset) as *mut T, value);
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
        unsafe {
            core::ptr::copy(slice.as_ptr(), ptr.add(offset), slice.len());
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
        let slice = unsafe { core::slice::from_raw_parts(ptr.add(offset), len) };
        Ok(Vec::from(slice))
    }

    pub fn read<T: Copy>(&self, isolation: &Isolation, offset: usize) -> Result<T, ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        assert!(matches!(isolation, Isolation::InKernel));
        unsafe { Ok(core::ptr::read(ptr.add(offset) as *const T)) }
    }
}
