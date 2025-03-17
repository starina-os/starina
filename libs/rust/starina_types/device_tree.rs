use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::slice;

use fdt_rs::base::*;
use fdt_rs::index::DevTreeIndex;
use fdt_rs::prelude::*;
use fdt_rs::spec::fdt_header;
use hashbrown::HashMap;
use serde::Deserialize;
use serde::Serialize;

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

#[derive(Debug)]
pub enum ParseError {
    InvalidMagicNumber(fdt_rs::error::DevTreeError),
    InvalidSize(fdt_rs::error::DevTreeError),
    InvalidTree(fdt_rs::error::DevTreeError),
    InvalidNode(fdt_rs::error::DevTreeError),
    InvalidName(fdt_rs::error::DevTreeError),
    InvalidProp(fdt_rs::error::DevTreeError),
    InvalidLayout(fdt_rs::error::DevTreeError),
    InvalidIndex(fdt_rs::error::DevTreeError),
}

fn stringlist_to_vec(
    prop: &fdt_rs::index::DevTreeIndexProp<'_, '_, '_>,
) -> Result<Vec<String>, ParseError> {
    let mut values = Vec::new();
    let mut iter = prop.iter_str();
    while let Some(s) = iter.next().map_err(ParseError::InvalidProp)? {
        values.push(s.to_owned());
    }

    Ok(values)
}

fn parse_reg(
    regs: &mut Vec<Reg>,
    prop: &fdt_rs::index::DevTreeIndexProp<'_, '_, '_>,
    found_bus: &mut FoundBus,
) -> Result<(), ParseError> {
    let mut i = 0;
    loop {
        if prop.u32(i).is_err() {
            // End of the regs.
            break;
        }

        let addr = match found_bus.address_cells {
            1 => prop.u32(i).map_err(ParseError::InvalidProp)? as u64,
            2 => {
                let high = prop.u32(i).map_err(ParseError::InvalidProp)? as u64;
                let low = prop.u32(i + 1).map_err(ParseError::InvalidProp)? as u64;
                (high << 32) | low
            }
            _ => {
                panic!("unsupported address cells: {}", found_bus.address_cells);
            }
        };
        i += found_bus.address_cells as usize;

        let size = match found_bus.size_cells {
            1 => prop.u32(i).map_err(ParseError::InvalidProp)? as u64,
            2 => {
                let high = prop.u32(i).map_err(ParseError::InvalidProp)? as u64;
                let low = prop.u32(i + 1).map_err(ParseError::InvalidProp)? as u64;
                (high << 32) | low
            }
            _ => {
                panic!("unsupported size cells: {}", found_bus.size_cells);
            }
        };
        i += found_bus.size_cells as usize;

        regs.push(Reg { addr, size });
    }

    Ok(())
}
struct FoundBus {
    is_referenced: bool,
    bus: BusNode,
    address_cells: u32,
    size_cells: u32,
}

impl DeviceTree {
    pub fn parse(dtb: *const u8) -> Result<Self, ParseError> {
        let devtree = unsafe {
            // Check  the magic number and read the size of the device tree.
            let dtb_magic = { slice::from_raw_parts(dtb, size_of::<fdt_header>()) };
            let size =
                DevTree::read_totalsize(dtb_magic).map_err(ParseError::InvalidMagicNumber)?;

            // Parse the device tree.
            let dtb = { slice::from_raw_parts(dtb, size) };
            DevTree::new(dtb).map_err(ParseError::InvalidTree)?
        };

        let layout = DevTreeIndex::get_layout(&devtree).map_err(ParseError::InvalidLayout)?;
        let mut vec = vec![0u8; layout.size() + layout.align()];
        let devtree_index =
            DevTreeIndex::new(devtree, vec.as_mut_slice()).map_err(ParseError::InvalidIndex)?;

        // Enumerate all buses.
        let mut found_buses = HashMap::new();
        for node in devtree_index.nodes() {
            let node_name = node.name().map_err(ParseError::InvalidName)?;
            let mut compatible = None;
            let mut address_cells = None;
            let mut size_cells = None;

            for prop in node.props() {
                let prop_name = prop.name().map_err(ParseError::InvalidName)?;
                match prop_name {
                    "compatible" => {
                        compatible = Some(stringlist_to_vec(&prop)?);
                    }
                    "#address-cells" => {
                        let value = prop.u32(0).map_err(ParseError::InvalidProp)?;
                        address_cells = Some(value);
                    }
                    "#size-cells" => {
                        let value = prop.u32(0).map_err(ParseError::InvalidProp)?;
                        size_cells = Some(value);
                    }
                    _ => {}
                }
            }

            let Some(compatible) = compatible else {
                continue;
            };
            let Some(address_cells) = address_cells else {
                continue;
            };
            let Some(size_cells) = size_cells else {
                continue;
            };

            if compatible.iter().any(|c| c == "simple-bus") {
                found_buses.insert(
                    node_name.to_owned(),
                    FoundBus {
                        is_referenced: false,
                        bus: BusNode::NoMmu,
                        address_cells,
                        size_cells,
                    },
                );
            }
        }

        let mut devices = HashMap::new();
        for node in devtree_index.nodes() {
            let Some(parent_name) = node.parent().and_then(|p| p.name().ok()) else {
                continue;
            };

            let Some(found_bus) = found_buses.get_mut(parent_name) else {
                // Not connected to a bus.
                continue;
            };

            let node_name = node.name().map_err(ParseError::InvalidName)?;
            let mut compatible = Vec::new();
            let mut reg = Vec::new();
            for prop in node.props() {
                let prop_name = prop.name().map_err(ParseError::InvalidName)?;
                match prop_name {
                    "compatible" => {
                        let mut iter = prop.iter_str();
                        while let Some(s) = iter.next().map_err(ParseError::InvalidProp)? {
                            compatible.push(s.to_owned());
                        }
                    }
                    "reg" => {
                        parse_reg(&mut reg, &prop, found_bus)?;
                    }
                    _ => {}
                }
            }

            found_bus.is_referenced = true;
            devices.insert(
                node_name.to_owned(),
                DeviceNode {
                    compatible,
                    bus: parent_name.to_owned(),
                    reg,
                },
            );
        }

        let mut buses = HashMap::new();
        for (name, bus) in found_buses {
            if bus.is_referenced {
                buses.insert(name, bus.bus);
            }
        }

        Ok(DeviceTree { devices, buses })
    }
}
