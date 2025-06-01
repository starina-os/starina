use core::fmt;

/// Represents a host-physical memory address.
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

    pub fn checked_add(self, offset: usize) -> Option<PAddr> {
        self.as_usize().checked_add(offset).map(PAddr::new)
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

/// Represents a guest-physical memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct GPAddr(usize);

impl GPAddr {
    pub const fn new(addr: usize) -> GPAddr {
        GPAddr(addr)
    }

    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub fn checked_add(self, offset: usize) -> Option<GPAddr> {
        self.as_usize().checked_add(offset).map(GPAddr::new)
    }

    pub fn checked_sub(self, offset: usize) -> Option<GPAddr> {
        self.as_usize().checked_sub(offset).map(GPAddr::new)
    }
}

impl fmt::Display for GPAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(target_pointer_width = "64") {
            write!(f, "{:016x}", self.as_usize())
        } else {
            write!(f, "{:08x}", self.as_usize())
        }
    }
}
