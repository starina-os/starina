use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use talc::ErrOnOom;
use talc::Span;
use talc::Talc;
use talc::Talck;

pub struct TalcAllocator {
    allocator: Talck<spin::Mutex<()>, ErrOnOom>,
}

impl TalcAllocator {
    pub const fn new() -> Self {
        Self {
            allocator: Talc::new(ErrOnOom).lock(),
        }
    }

    pub fn add_region(&self, heap: *mut u8, heap_len: usize) {
        unsafe {
            let _ = self
                .allocator
                .lock()
                .claim(Span::from_base_size(heap, heap_len));
        }
    }
}

unsafe impl GlobalAlloc for TalcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { self.allocator.alloc(layout) };
        if result.is_null() {
            panic!("out of memory");
        }

        result
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        debug_assert!(!ptr.is_null());
        debug_assert!(layout.size() > 0);

        unsafe {
            self.allocator.dealloc(ptr, layout);
        }
    }
}
