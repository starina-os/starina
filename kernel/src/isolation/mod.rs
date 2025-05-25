use alloc::vec::Vec;

use starina_types::error::ErrorCode;

pub enum IsolationHeap {
    InKernel { ptr: *const u8, len: usize },
}

pub enum IsolationHeapMut {
    InKernel { ptr: *mut u8, len: usize },
}

impl IsolationHeap {
    pub fn read_to_vec(&self, offset: usize, len: usize) -> Result<Vec<u8>, ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        let slice = unsafe { core::slice::from_raw_parts(ptr.add(offset), len) };

        let mut buf = Vec::new();
        buf.try_reserve_exact(len)
            .map_err(|_| ErrorCode::OutOfMemory)?;
        buf.extend_from_slice(slice);

        Ok(buf)
    }

    pub fn read<T: Copy>(&self, offset: usize) -> Result<T, ErrorCode> {
        let IsolationHeap::InKernel { ptr, .. } = self;
        unsafe { Ok(core::ptr::read(ptr.add(offset) as *const T)) }
    }
}

impl IsolationHeapMut {
    pub fn read<T: Copy>(&self, offset: usize) -> Result<T, ErrorCode> {
        let IsolationHeapMut::InKernel { ptr, .. } = self;
        unsafe { Ok(core::ptr::read(ptr.add(offset) as *const T)) }
    }

    pub fn write<T: Copy>(&mut self, offset: usize, value: T) -> Result<(), ErrorCode> {
        let IsolationHeapMut::InKernel { ptr, .. } = self;
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
        isolation: &IsolationTy,
        offset: usize,
        slice: &[u8],
    ) -> Result<(), ErrorCode> {
        assert!(matches!(isolation, IsolationTy::InKernel));
        let IsolationHeapMut::InKernel { ptr, .. } = self;
        unsafe {
            core::ptr::copy(slice.as_ptr(), ptr.add(offset), slice.len());
        }
        Ok(())
    }
}

unsafe impl Send for IsolationHeap {}
unsafe impl Send for IsolationHeapMut {}
