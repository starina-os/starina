use core::slice;

use starina::error::ErrorCode;

use super::Isolation;
use super::IsolationPtr;
use crate::refcount::RefCounted;
use crate::refcount::SharedRef;

pub struct InKernel {
    _private: (),
}

impl InKernel {
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

impl Isolation for InKernel {
    fn read_bytes(&self, ptr: IsolationPtr, dst: &mut [u8]) -> Result<(), ErrorCode> {
        let raw_ptr = ptr.0 as *const u8;
        let src = unsafe { slice::from_raw_parts(raw_ptr, dst.len()) };
        dst.copy_from_slice(src);
        Ok(())
    }

    fn write_bytes(&self, ptr: IsolationPtr, src: &[u8]) -> Result<(), ErrorCode> {
        let raw_ptr = ptr.0 as *mut u8;
        let dst = unsafe { slice::from_raw_parts_mut(raw_ptr, src.len()) };
        dst.copy_from_slice(src);
        Ok(())
    }
}

pub static INKERNEL_ISOLATION: SharedRef<dyn Isolation> = {
    static INNER: RefCounted<InKernel> = RefCounted::new(InKernel::new());
    let isolation = unsafe { SharedRef::new_static(&INNER) };
    isolation as SharedRef<dyn Isolation>
};
