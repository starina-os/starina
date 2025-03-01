use core::ops::BitAnd;
use core::ops::BitAndAssign;
use core::ops::BitOr;
use core::ops::BitOrAssign;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Readiness(i8);

impl Readiness {
    pub const CLOSED: Readiness = Readiness(1 << 0);
    pub const READABLE: Readiness = Readiness(1 << 1);
    pub const WRITABLE: Readiness = Readiness(1 << 2);

    pub const fn new() -> Readiness {
        Readiness(0)
    }

    pub fn contains(&self, other: Readiness) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for Readiness {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Readiness(self.0 | rhs.0)
    }
}

impl BitAnd for Readiness {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Readiness(self.0 & rhs.0)
    }
}

impl BitOrAssign for Readiness {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAndAssign for Readiness {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
