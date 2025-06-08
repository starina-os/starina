/// Represents a time from a certain point in the past, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonotonicTime(u64);

impl MonotonicTime {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * 1_000_000)
    }

    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    pub const fn as_nanos(&self) -> u64 {
        self.0
    }
}
