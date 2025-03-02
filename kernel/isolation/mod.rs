use alloc::vec::Vec;

use starina::error::ErrorCode;

pub enum Isolation {
    InKernel,
}

pub enum IsolationHeap {
    InKernel { ptr: *const u8, len: usize },
}

pub enum IsolationHeapMut {
    InKernel { ptr: *mut u8, len: usize },
}
impl IsolationHeap {
    pub fn read_to_vec(
        &self,
        isolation: &Isolation,
        offset: usize,
        len: usize,
    ) -> Result<Vec<u8>, ErrorCode> {
        assert!(matches!(isolation, Isolation::InKernel));
        let IsolationHeap::InKernel { ptr, .. } = self;
        let slice = unsafe { core::slice::from_raw_parts(ptr.add(offset), len) };

        let mut buf = Vec::new();
        buf.try_reserve_exact(len)
            .map_err(|_| ErrorCode::OutOfMemory)?;
        buf.extend_from_slice(slice);

        Ok(buf)
    }

    pub fn read<T: Copy>(&self, isolation: &Isolation, offset: usize) -> Result<T, ErrorCode> {
        assert!(matches!(isolation, Isolation::InKernel));
        let IsolationHeap::InKernel { ptr, .. } = self;
        unsafe { Ok(core::ptr::read(ptr.add(offset) as *const T)) }
    }
}

impl IsolationHeapMut {
    pub fn write<T: Copy>(
        &mut self,
        isolation: &Isolation,
        offset: usize,
        value: T,
    ) -> Result<(), ErrorCode> {
        assert!(matches!(isolation, Isolation::InKernel));
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
        isolation: &Isolation,
        offset: usize,
        slice: &[u8],
    ) -> Result<(), ErrorCode> {
        assert!(matches!(isolation, Isolation::InKernel));
        let IsolationHeapMut::InKernel { ptr, .. } = self;
        unsafe {
            core::ptr::copy(slice.as_ptr(), ptr.add(offset), slice.len());
        }
        Ok(())
    }
}
