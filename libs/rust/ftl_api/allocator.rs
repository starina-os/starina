use core::ptr::addr_of;

use linked_list_allocator::LockedHeap;

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub(crate) fn init() {
    extern "C" {
        static __heap: u8;
        static __heap_end: u8;
    }

    unsafe {
        let heap_start = addr_of!(__heap) as usize;
        let heap_end = addr_of!(__heap_end) as usize;
        GLOBAL_ALLOCATOR
            .lock()
            .init(heap_start as *mut u8, heap_end - heap_start);
    }
}
