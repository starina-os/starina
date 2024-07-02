use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::alloc::LayoutError;
use core::num::NonZeroUsize;
use core::ptr::NonNull;

use ftl_bump_allocator::BumpAllocator;
use ftl_types::address::VAddr;
use ftl_utils::alignment::is_aligned;

use crate::arch::PAGE_SIZE;
use crate::boot::BootInfo;
use crate::boot::FreeMem;
use crate::spinlock::SpinLock;

#[global_allocator]
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

fn alloc_layout_for_pages(len: usize) -> Result<Layout, LayoutError> {
    debug_assert!(is_aligned(len, PAGE_SIZE));
    debug_assert!(len > 0);

    Layout::from_size_align(len, PAGE_SIZE)
}

#[derive(Debug, PartialEq, Eq)]
pub enum AllocPagesError {
    InvalidLayout(LayoutError),
}

pub struct AllocatedPages {
    base: NonNull<u8>,
    len: usize,
}

// FIXME:
unsafe impl Sync for AllocatedPages {}
unsafe impl Send for AllocatedPages {}

impl AllocatedPages {
    /// Allocate memory pages always accessible from the kernel's address space.
    pub fn alloc(len: usize) -> Result<AllocatedPages, AllocPagesError> {
        let layout = alloc_layout_for_pages(len).map_err(AllocPagesError::InvalidLayout)?;

        // SAFETY: `len` is not zero as checked above. I hope that's also true in
        //         the release build too.
        let ptr = unsafe { GLOBAL_ALLOCATOR.alloc(layout) };

        Ok(AllocatedPages {
            base: NonNull::new(ptr).unwrap(),
            len,
        })
    }

    pub fn as_vaddr(&self) -> VAddr {
        // SAFETY: NonNull guarantees it's non-zero.
        let nonzero = unsafe { NonZeroUsize::new_unchecked(self.base.as_ptr() as usize) };

        VAddr::from_nonzero(nonzero)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.base.as_ptr()
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_ptr(), self.len) }
    }
}

impl Drop for AllocatedPages {
    fn drop(&mut self) {
        let layout = alloc_layout_for_pages(self.len).unwrap();
        // SAFETY: This object owns the memory region and `layout` is calculated
        //         exactly same as the one used in `alloc_kernel_pages`.
        unsafe {
            GLOBAL_ALLOCATOR.dealloc(self.base.as_ptr(), layout);
        }
    }
}

/// Initializes the memory subsystem.
///
/// After this function is called, the global allocator (e.g. `Box`, `Vec`, etc.)
/// becomes available.
pub fn init(bootinfo: &BootInfo) {
    for FreeMem { start, size } in bootinfo.free_mems.iter() {
        println!(
            "free memory: 0x{:016x} - 0x{:016x} ({})",
            start,
            start + size.in_bytes(),
            size
        );

        GLOBAL_ALLOCATOR.add_region(*start as *mut u8, size.in_bytes());
    }
}
