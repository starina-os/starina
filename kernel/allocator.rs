use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::num::NonZeroUsize;

use crate::spinlock::SpinLock;

#[cfg_attr(target_os = "none", global_allocator)]
#[cfg_attr(not(target_os = "none"), allow(unused))]
pub static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

/// The default in-kernel memory allocator.
///
/// Allocated memory are always accessible from the kernel's address space,
/// that is, memory pages added to this allocator must not be swapped out,
/// or something like that.
pub struct GlobalAllocator {
    inner: SpinLock<BumpAllocator>,
}

impl GlobalAllocator {
    /// Creates a new global allocator.
    ///
    /// The allocator is initially empty. Memory regions must be added
    /// by calling [`GlobalAllocator::add_region`] method.
    pub const fn new() -> GlobalAllocator {
        let allocator = BumpAllocator::new();

        GlobalAllocator {
            inner: SpinLock::new(allocator),
        }
    }

    /// Adds a new memory region to the allocator.
    ///
    /// The memory region must be always mapped to the kernel's address space.
    pub fn add_region(&self, heap: *mut u8, heap_len: usize) {
        self.inner.lock().add_region(heap as usize, heap_len);
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let addr = self
            .inner
            .lock()
            .allocate(layout.size(), layout.align())
            .expect("failed to allocate memory");

        addr.get() as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* We can't deallocate. This is well-known limitation of bump allocator! */
    }
}

const fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value) & !(align - 1)
}

/// A bump memory allocator.
///
/// Unlike typical allocators, this allocator does not support freeing memory.
/// Instead, it only supports allocating memory. This makes it extremely fast
/// and simple.
///
/// Typically, this allocator is used for allocating memory in initialization
/// phase such that the allocated memory is never freed.
pub struct BumpAllocator {
    top: usize,
    bottom: usize,
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl BumpAllocator {
    // Creates a new bump allocator. Initially, the allocator has no memory
    // region. Call `add_region` to add a memory region.
    pub const fn new() -> BumpAllocator {
        BumpAllocator { bottom: 0, top: 0 }
    }

    // Gives a meory region `[base, base + len)` to the allocator.
    // `base` must be non-zero.
    pub fn add_region(&mut self, base: usize, len: usize) {
        debug_assert!(self.bottom == 0, "only one region is supported");
        debug_assert!(base > 0);

        self.bottom = base;
        self.top = base + len;
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

    /// Allocates all remaining memory with the given `align` bytes alignment.
    ///
    /// If any memory is allocated, returns the beginning address and the size
    /// of the allocated memory.
    #[track_caller]
    pub fn allocate_all(&mut self, align: usize) -> Option<(NonZeroUsize, usize)> {
        self.top = align_down(self.top, align);
        if self.top < self.bottom {
            return None;
        }

        let size = self.top - self.bottom;
        self.top = self.bottom;

        // SAFETY: `self.bottom` is checked to be non-zero.
        unsafe { Some((NonZeroUsize::new_unchecked(self.bottom), size)) }
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
        let mut allocator = BumpAllocator::new();
        allocator.add_region(0x20000, 0x4000);
        assert_eq!(allocator.allocate(0, 0x1000), None);
    }

    #[test]
    fn test_bump_allocator() {
        let mut allocator = BumpAllocator::new();
        allocator.add_region(0x20000, 0x4000);
        assert_eq!(allocator.allocate(0x1000, 0x1000), Some(nonzero(0x23000)));
        assert_eq!(allocator.allocate(0x1000, 0x1000), Some(nonzero(0x22000)));
        assert_eq!(allocator.allocate(0xf00, 0x1000), Some(nonzero(0x21000)));
        assert_eq!(allocator.allocate(0x1000, 0x1000), Some(nonzero(0x20000)));
        assert_eq!(allocator.allocate(0x1000, 0x1000), None);
    }
}
