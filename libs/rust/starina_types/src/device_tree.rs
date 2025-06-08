use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;
use serde::Deserialize;
use serde::Serialize;

use crate::interrupt::IrqMatcher;

/// The device tree. This is the root of the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceTree {
    pub devices: HashMap<String, DeviceNode>,
    pub timer_freq: u64,
}

/// A node in the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceNode {
    pub compatible: Vec<String>,
    pub reg: Vec<Reg>,
    pub interrupts: Vec<IrqMatcher>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Reg {
    pub addr: u64,
    pub size: u64,
}
