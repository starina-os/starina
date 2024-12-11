use core::ops::{ BitAnd, BitOr};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct PollEvent(u8);

impl PollEvent {
    pub const CLOSED: PollEvent = PollEvent::from_raw(1 << 0);
    pub const READABLE: PollEvent = PollEvent::from_raw(1 << 1);
    pub const WRITABLE: PollEvent = PollEvent::from_raw(1 << 2);

    pub const fn zeroed() -> PollEvent {
        PollEvent(0)
    }

    pub const fn from_raw(value: u8) -> PollEvent {
        PollEvent(value)
    }

    pub fn as_raw(&self) -> u8 {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn contains(&self, other: PollEvent) -> bool {
        (self.0 & other.0) != 0
    }
}

impl BitOr for PollEvent {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        PollEvent(self.0 | rhs.0)
    }
}

impl BitAnd for PollEvent {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        PollEvent(self.0 & rhs.0)
    }
}
