use alloc::vec::Vec;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]

/// Userspace address.
///
/// FIXME: The current implementation assumes in-kernel apps only.
pub struct UAddr(usize);

impl UAddr {
    pub const fn new(addr: usize) -> UAddr {
        UAddr(addr)
    }

    pub fn from_kernel_ptr<T>(ptr: *const T) -> UAddr {
        UAddr(ptr as usize)
    }

    pub fn read_from_user<T: Copy>(&self) -> T {
        unsafe { core::ptr::read(self.0 as *const T) }
    }

    pub fn read_from_user_at<T: Copy>(&self, offset: usize) -> T {
        unsafe { core::ptr::read((self.0 + offset) as *const T) }
    }

    pub fn read_from_user_to_vec<T: Copy>(&self, offset: usize, len: usize) -> Vec<T> {
        unsafe { core::slice::from_raw_parts((self.0 + offset) as *const T, len).to_vec() }
    }

    pub fn write_to_user_at<T: Copy>(&self, offset: usize, value: T) {
        unsafe { core::ptr::write((self.0 + offset) as *mut T, value) }
    }

    pub fn write_to_user_at_slice<T: Copy>(&self, offset: usize, slice: &[T]) {
        unsafe {
            let dest = core::slice::from_raw_parts_mut((self.0 + offset) as *mut T, slice.len());
            dest.copy_from_slice(slice);
        }
    }
}
