use serde::Deserialize;
use serde::Serialize;

use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Irq(u32);

impl Irq {
    pub const fn new(irq: u32) -> Self {
        Self(irq)
    }

    pub fn from_raw_isize(raw: isize) -> Result<Self, ErrorCode> {
        match u32::try_from(raw) {
            Ok(raw) => Ok(Self(raw as u32)),
            Err(_) => Err(ErrorCode::InvalidArg),
        }
    }

    pub const fn as_raw(&self) -> u32 {
        self.0
    }
}
