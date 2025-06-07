/// Represents a time from a certain point in the past, in nanoseconds.
pub struct MonotonicTime(u64);

impl MonotonicTime {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}
