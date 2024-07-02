use core::fmt;
use core::num::NonZeroUsize;
use core::ptr;

/// Represents a physical memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct PAddr(NonZeroUsize);

impl PAddr {
    pub const fn new(addr: usize) -> Option<PAddr> {
        // We can't use `Option::map` here because it's not a const fn.
        match NonZeroUsize::new(addr) {
            Some(addr) => Some(PAddr(addr)),
            None => None,
        }
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0.get()
    }

    #[inline(always)]
    pub const fn as_nonzero(self) -> NonZeroUsize {
        self.0
    }

    #[must_use]
    pub fn add(&self, offset: usize) -> PAddr {
        PAddr::new(self.0.get() + offset).unwrap()
    }
}

impl fmt::Display for PAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(target_pointer_width = "64") {
            write!(f, "{:016x}", self.as_usize())
        } else {
            write!(f, "{:08x}", self.as_usize())
        }
    }
}

/// Represents a *kernel* virtual memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct VAddr(NonZeroUsize);

impl VAddr {
    pub const fn new(addr: usize) -> Option<VAddr> {
        // We can't use `Option::map` here because it's not a const fn.
        match NonZeroUsize::new(addr) {
            Some(addr) => Some(VAddr(addr)),
            None => None,
        }
    }

    pub const fn from_nonzero(addr: NonZeroUsize) -> VAddr {
        VAddr(addr)
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0.get()
    }

    #[inline(always)]
    pub const fn as_nonzero(self) -> NonZeroUsize {
        self.0
    }

    pub const fn as_ptr<T>(self) -> *const T {
        self.as_usize() as *const _
    }

    pub const fn as_mut_ptr<T>(self) -> *mut T {
        // TODO: arch-specific check
        self.as_usize() as *mut _
    }

    /// # Safety
    /// See <https://doc.rust-lang.org/std/ptr/fn.read_volatile.html>.
    pub unsafe fn read_volatile<T: Copy>(self) -> T {
        ptr::read_volatile(self.as_ptr::<T>())
    }

    /// # Safety
    /// See <https://doc.rust-lang.org/std/ptr/fn.write_volatile.html>.
    pub unsafe fn write_volatile<T: Copy>(self, value: T) {
        ptr::write_volatile(self.as_mut_ptr(), value);
    }

    #[must_use]
    pub fn add(&self, offset: usize) -> VAddr {
        VAddr::from_nonzero(self.0.checked_add(offset).unwrap())
    }
}

impl fmt::Display for VAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.as_usize())
    }
}
