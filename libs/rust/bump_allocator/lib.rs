//! A bump memory allocator.
//!
//! Unlike typical allocators, this allocator does not support freeing memory.
//! Instead, it only supports allocating memory. This makes it extremely fast
//! and simple.
//!
//! Typically, this allocator is used for allocating memory in initialization
//! phase such that the allocated memory is never freed.
//!
//! # Examples
//!
//! ```
//! use bump_allocator::BumpAllocator;
//!
//! let mut allocator = BumpAllocator::new();
//! allocator.add_region(0x20000 /* base */, 0x1000 /* len */);
//! let addr1 = allocator.allocate(0x100 /* size */, 0x100 /* align */);
//! ```
#![no_std]
#![cfg_attr(test, feature(test))]

use core::num::NonZeroUsize;

const fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value) & !(align - 1)
}

/// A bump memory allocator.
pub struct BumpAllocator {
    top: usize,
    bottom: usize,
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
