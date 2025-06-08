use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MonotonicTime(u64);

impl MonotonicTime {
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    pub const fn as_nanos(&self) -> u64 {
        self.0
    }

    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }
}
