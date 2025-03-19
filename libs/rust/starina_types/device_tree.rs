use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;
use serde::Deserialize;
use serde::Serialize;

use crate::interrupt::Irq;

/// The device tree. This is the root of the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceTree {
    pub buses: HashMap<String, BusNode>,
    pub devices: HashMap<String, DeviceNode>,
}

/// A node in the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceNode {
    pub compatible: Vec<String>,
    pub bus: String,
    pub reg: Vec<Reg>,
    pub interrupts: Vec<InterruptDesc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InterruptDesc {
    Static(Irq),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum BusNode {
    NoMmu,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Reg {
    pub addr: u64,
    pub size: u64,
}
