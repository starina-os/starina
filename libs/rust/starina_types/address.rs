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
