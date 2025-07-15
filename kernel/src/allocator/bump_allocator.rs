use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::num::NonZeroUsize;

use arrayvec::ArrayVec;

use crate::spinlock::SpinLock;

/// A bump memory allocator.
///
/// Unlike typical allocators, this allocator does not support freeing memory.
/// Instead, it only supports allocating memory. This makes it extremely fast
/// and simple.
///
/// Typically, this allocator is used for allocating memory in initialization
/// phase such that the allocated memory is never freed.
pub struct BumpAllocator {
    regions: SpinLock<ArrayVec<Region, 4>>,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            regions: SpinLock::new(ArrayVec::new_const()),
        }
    }

    pub fn add_region(&self, heap: *mut u8, heap_len: usize) {
        self.regions
            .lock()
            .try_push(Region::new(heap as usize, heap_len))
            .expect("too many memory regions");
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        for region in self.regions.lock().iter_mut() {
            if let Some(addr) = region.allocate(layout.size(), layout.align()) {
                return addr.get() as *mut u8;
            }
        }
        panic!("out of memory");
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* We can't deallocate. This is well-known limitation of bump allocator! */
    }
}

const fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value) & !(align - 1)
}

struct Region {
    top: usize,
    bottom: usize,
}

impl Region {
    pub const fn new(base: usize, len: usize) -> Region {
        Region {
            bottom: base,
            top: base + len,
        }
    }

    /// Allocates `size` bytes of memory with the given `align` bytes alignment.
    /// Returns the beginning address of the allocated memory if successful.
    #[track_caller]
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<NonZeroUsize> {
        if size == 0 {
            return None;
        }

        let new_top = align_down(self.top.checked_sub(size)?, align);
        if new_top < self.bottom {
            return None;
        }

        self.top = new_top;

        // SAFETY: `self.top` is checked to be larger than `self.bottom`.
        unsafe { Some(NonZeroUsize::new_unchecked(self.top)) }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroUsize;

    use super::*;

    fn nonzero(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).unwrap()
    }

    #[test]
    fn test_zero_size() {
        let mut region = Region::new(0x20000, 0x4000);
        assert_eq!(region.allocate(0, 0x1000), None);
    }

    #[test]
    fn test_bump_allocator() {
        let mut region = Region::new(0x20000, 0x4000);
        assert_eq!(region.allocate(0x1000, 0x1000), Some(nonzero(0x23000)));
        assert_eq!(region.allocate(0x1000, 0x1000), Some(nonzero(0x22000)));
        assert_eq!(region.allocate(0xf00, 0x1000), Some(nonzero(0x21000)));
        assert_eq!(region.allocate(0x1000, 0x1000), Some(nonzero(0x20000)));
        assert_eq!(region.allocate(0x1000, 0x1000), None);
    }
}
