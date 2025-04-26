use core::fmt;

/// Represents a physical memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct PAddr(usize);

impl PAddr {
    pub const fn new(addr: usize) -> PAddr {
        PAddr(addr)
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    // TODO: Check overflow.
    pub fn add(self, offset: usize) -> PAddr {
        PAddr::new(self.as_usize() + offset)
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

/// Represents a virtual memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct VAddr(usize);

impl VAddr {
    pub const fn new(addr: usize) -> VAddr {
        VAddr(addr)
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub fn add(self, offset: usize) -> VAddr {
        // TODO: Check overflow.
        VAddr::new(self.as_usize() + offset)
    }

    //// # Safety
    ///
    /// <https://doc.rust-lang.org/std/ptr/index.html#pointer-to-reference-conversion>
    pub unsafe fn as_mut_ptr<T>(self) -> *mut T {
        let ptr = self.as_usize() as *mut T;
        unsafe { &mut *ptr }
    }

    //// # Safety
    ///
    /// <https://doc.rust-lang.org/std/ptr/index.html#pointer-to-reference-conversion>
    pub unsafe fn as_ptr<T>(self) -> *const T {
        let ptr = self.as_usize() as *const T;
        unsafe { &*ptr }
    }
}

impl fmt::Display for VAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(target_pointer_width = "64") {
            write!(f, "{:016x}", self.as_usize())
        } else {
            write!(f, "{:08x}", self.as_usize())
        }
    }
}

/// Represents a device-visible memory address.
///
/// Typically it is equal to the physical address, but it can be different
/// in some cases, e.g. when using IOMMU.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct DAddr(usize);

impl DAddr {
    pub const fn new(addr: usize) -> DAddr {
        DAddr(addr)
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub fn add(self, offset: usize) -> DAddr {
        // TODO: Check overflow.
        DAddr::new(self.as_usize() + offset)
    }
}

impl fmt::Display for DAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(target_pointer_width = "64") {
            write!(f, "{:016x}", self.as_usize())
        } else {
            write!(f, "{:08x}", self.as_usize())
        }
    }
}
