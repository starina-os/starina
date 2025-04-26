use serde::Deserialize;
use serde::Serialize;

use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Irq(u32);

impl Irq {
    pub const fn from_raw(irq: u32) -> Self {
        Self(irq)
    }

    pub fn from_raw_isize(raw: isize) -> Result<Self, ErrorCode> {
        match u32::try_from(raw) {
            Ok(raw) => Ok(Self(raw)),
            Err(_) => Err(ErrorCode::InvalidArg),
        }
    }

    pub const fn as_raw(&self) -> u32 {
        self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum IrqMatcher {
    Static(Irq),
}

impl IrqMatcher {
    pub const fn as_raw(&self) -> u32 {
        match self {
            IrqMatcher::Static(irq) => {
                // Upper bits are reserved for future use.
                assert!(irq.as_raw() < 4096);
                irq.as_raw()
            }
        }
    }

    pub fn from_raw_isize(raw: isize) -> Result<Self, ErrorCode> {
        match u32::try_from(raw) {
            Ok(raw) if raw < 4096 => Ok(Self::Static(Irq::from_raw(raw))),
            _ => Err(ErrorCode::InvalidArg),
        }
    }
}
