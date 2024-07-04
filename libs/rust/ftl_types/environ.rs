use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::Deserialize;
use serde::Serialize;

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct Device {
    pub name: String,
    pub compatible: String,
    pub reg: u64,
    pub interrupts: Option<Vec<u32>>,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("name", &self.name)
            .field("compatible", &self.compatible)
            .field("reg", &format_args!("{:#08x}", self.reg))
            .field("interrupts", &self.interrupts)
            .finish()
    }
}
