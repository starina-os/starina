use crate::isolation::Isolation;
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
    fn isolation_heap(&self, ptr: super::IsolationPtr, len: usize) -> super::IsolationHeap {
        todo!()
    }

    fn isolation_heap_mut(
        &mut self,
        ptr: super::IsolationPtr,
        len: usize,
    ) -> super::IsolationHeapMut {
        todo!()
    }
}

pub static INKERNEL_ISOLATION: SharedRef<dyn Isolation> = {
    static INNER: RefCounted<InKernel> = RefCounted::new(InKernel::new());
    let isolation = unsafe { SharedRef::new_static(&INNER) };
    isolation as SharedRef<dyn Isolation>
};
