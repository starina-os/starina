use alloc::vec::Vec;

pub enum Isolation {
    InKernel,
}

pub struct IsolationHeap {
    ptr: usize,
    len: usize,
}

impl IsolationHeap {
    pub fn write<T>(&mut self, isolation: &Isolation, offset: usize, value: T) {
        assert!(matches!(isolation, Isolation::InKernel));

        let ptr = self.ptr + offset;
        unsafe {
            core::ptr::write(ptr as *mut T, value);
        }
    }

    pub fn read_to_vec(&self, isolation: &Isolation, offset: usize, len: usize) -> Vec<u8> {
        assert!(matches!(isolation, Isolation::InKernel));

        let ptr = self.ptr + offset;
        let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
        slice.to_vec()
    }

    pub fn read<T>(&self, isolation: &Isolation, offset: usize) -> T {
        assert!(matches!(isolation, Isolation::InKernel));

        let ptr = self.ptr + offset;
        unsafe { core::ptr::read(ptr as *const T) }
    }
}
