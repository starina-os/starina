use core::fmt;
use core::fmt::Write;
use core::ops::BitAnd;
use core::ops::BitAndAssign;
use core::ops::BitOr;
use core::ops::BitOrAssign;

use crate::error::ErrorCode;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Readiness(u8);

impl Default for Readiness {
    fn default() -> Self {
        Self::new()
    }
}

impl Readiness {
    pub const CLOSED: Readiness = Readiness(1 << 0);
    pub const READABLE: Readiness = Readiness(1 << 1);
    pub const WRITABLE: Readiness = Readiness(1 << 2);
    pub const ALL: Readiness = Readiness(0xff);
    pub const NONE: Readiness = Readiness(0);

    pub fn new() -> Readiness {
        Readiness(0)
    }

    pub const fn from_raw(raw: u8) -> Readiness {
        Readiness(raw)
    }

    pub fn from_raw_isize(raw: isize) -> Result<Readiness, ErrorCode> {
        match u8::try_from(raw) {
            Ok(raw) => Ok(Readiness::from_raw(raw)),
            _ => Err(ErrorCode::InvalidArg),
        }
    }

    pub fn as_isize(&self) -> isize {
        self.0 as isize
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn contains(&self, other: Readiness) -> bool {
        self.0 & other.0 != 0
    }

    pub fn invert(&self) -> Readiness {
        Readiness(!self.0)
    }
}

impl fmt::Debug for Readiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.contains(Readiness::CLOSED) {
            f.write_char('C')?;
        }
        if self.contains(Readiness::READABLE) {
            f.write_char('R')?;
        }
        if self.contains(Readiness::WRITABLE) {
            f.write_char('W')?;
        }

        Ok(())
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
