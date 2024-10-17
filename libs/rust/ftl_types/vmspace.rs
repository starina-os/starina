use core::ops;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct PageProtect(u8);

impl PageProtect {
    pub const READABLE: PageProtect = PageProtect::from_raw(1 << 1);
    pub const WRITABLE: PageProtect = PageProtect::from_raw(1 << 2);
    pub const EXECUTABLE: PageProtect = PageProtect::from_raw(1 << 3);
    pub const USER: PageProtect = PageProtect::from_raw(1 << 4);

    pub const fn zeroed() -> PageProtect {
        PageProtect(0)
    }

    pub const fn from_raw(value: u8) -> PageProtect {
        PageProtect(value)
    }

    pub fn contains(&self, other: PageProtect) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn user_allowed_flags(&self) -> bool {
        let allowed = Self::READABLE | Self::WRITABLE | Self::EXECUTABLE;
        (self.0 & allowed.0) == self.0
    }

    pub fn as_raw(&self) -> u8 {
        self.0
    }
}

impl ops::BitOr for PageProtect {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        PageProtect(self.0 | rhs.0)
    }
}

impl ops::BitAnd for PageProtect {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        PageProtect(self.0 & rhs.0)
    }
}

impl ops::BitOrAssign for PageProtect {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl ops::BitAndAssign for PageProtect {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
