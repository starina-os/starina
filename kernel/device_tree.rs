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
use starina::device_tree::BusNode;
use starina::device_tree::DeviceNode;
use starina::device_tree::DeviceTree;
use starina::device_tree::Reg;
use starina::interrupt::IrqMatcher;

use crate::arch::INTERRUPT_CONTROLLER;

fn stringlist_to_vec(
    prop: &fdt_rs::index::DevTreeIndexProp<'_, '_, '_>,
) -> Result<Vec<String>, fdt_rs::error::DevTreeError> {
    let mut values = Vec::new();
    let mut iter = prop.iter_str();
    while let Some(s) = iter.next()? {
        values.push(s.to_owned());
    }

    Ok(values)
}

fn parse_reg(
    regs: &mut Vec<Reg>,
    prop: &fdt_rs::index::DevTreeIndexProp<'_, '_, '_>,
    found_bus: &mut FoundBus,
) -> Result<(), fdt_rs::error::DevTreeError> {
    let mut i = 0;
    loop {
        if prop.u32(i).is_err() {
            // End of the regs.
            break;
        }

        let addr = match found_bus.address_cells {
            1 => prop.u32(i)? as u64,
            2 => {
                let high = prop.u32(i)? as u64;
                let low = prop.u32(i + 1)? as u64;
                (high << 32) | low
            }
            _ => {
                panic!("unsupported address cells: {}", found_bus.address_cells);
            }
        };
        i += found_bus.address_cells as usize;

        let size = match found_bus.size_cells {
            1 => prop.u32(i)? as u64,
            2 => {
                let high = prop.u32(i)? as u64;
                let low = prop.u32(i + 1)? as u64;
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

#[derive(Debug)]
struct FoundBus {
    is_referenced: bool,
    bus: BusNode,
    address_cells: u32,
    size_cells: u32,
}

#[derive(Debug)]
struct FoundInterruptController {
    name: String,
    is_compatible: bool,
    interrupt_cells: u32,
}

pub fn parse(dtb: *const u8) -> Result<DeviceTree, fdt_rs::error::DevTreeError> {
    let devtree = unsafe {
        // Check  the magic number and read the size of the device tree.
        let dtb_magic = { slice::from_raw_parts(dtb, size_of::<fdt_header>()) };
        let size = DevTree::read_totalsize(dtb_magic)?;

        // Parse the device tree.
        let dtb = { slice::from_raw_parts(dtb, size) };
        DevTree::new(dtb)?
    };

    let layout = DevTreeIndex::get_layout(&devtree)?;
    let mut vec = vec![0u8; layout.size() + layout.align()];
    let devtree_index = DevTreeIndex::new(devtree, vec.as_mut_slice())?;

    // Enumerate all buses.
    let mut found_buses = HashMap::new();
    for node in devtree_index.nodes() {
        let node_name = node.name()?;
        let mut compatible = None;
        let mut address_cells = None;
        let mut size_cells = None;

        for prop in node.props() {
            let prop_name = prop.name()?;
            match prop_name {
                "compatible" => {
                    compatible = Some(stringlist_to_vec(&prop)?);
                }
                "#address-cells" => {
                    let value = prop.u32(0)?;
                    address_cells = Some(value);
                }
                "#size-cells" => {
                    let value = prop.u32(0)?;
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

    // Enumerate all interrupt controllers.
    let mut found_intcs = HashMap::new();
    for node in devtree_index.nodes() {
        let Some(parent_name) = node.parent().and_then(|p| p.name().ok()) else {
            continue;
        };

        let Some(found_bus) = found_buses.get_mut(parent_name) else {
            // Not connected to a bus.
            continue;
        };

        let node_name = node.name()?;
        let mut is_interrupt_controller = false;
        let mut interrupt_cells = None;
        let mut compatible = Vec::new();
        let mut reg = Vec::new();
        let mut phandle = None;
        for prop in node.props() {
            let prop_name = prop.name()?;
            match prop_name {
                "compatible" => {
                    compatible = stringlist_to_vec(&prop)?;
                }
                "reg" => {
                    parse_reg(&mut reg, &prop, found_bus)?;
                }
                "interrupt-controller" => {
                    is_interrupt_controller = true;
                }
                "#interrupt-cells" => {
                    interrupt_cells = Some(prop.u32(0)?);
                }
                "phandle" => {
                    phandle = Some(prop.u32(0)?);
                }
                _ => {}
            }
        }

        if !is_interrupt_controller {
            continue;
        }

        let Some(phandle) = phandle else {
            continue;
        };

        let Some(interrupt_cells) = interrupt_cells else {
            continue;
        };

        if compatible.is_empty() {
            continue;
        }

        let is_compatible = INTERRUPT_CONTROLLER.try_init(&compatible, &reg).is_ok();
        found_intcs.insert(
            phandle,
            FoundInterruptController {
                name: node_name.to_owned(),
                is_compatible,
                interrupt_cells,
            },
        );
    }

    // Enumerate all devices.
    let mut devices = HashMap::new();
    for node in devtree_index.nodes() {
        let Some(parent_name) = node.parent().and_then(|p| p.name().ok()) else {
            continue;
        };

        let Some(found_bus) = found_buses.get_mut(parent_name) else {
            // Not connected to a bus.
            continue;
        };

        let node_name = node.name()?;
        let mut compatible = Vec::new();
        let mut reg = Vec::new();
        let mut interrupts = Vec::new();
        for prop in node.props() {
            let prop_name = prop.name()?;
            match prop_name {
                "compatible" => {
                    let mut iter = prop.iter_str();
                    while let Some(s) = iter.next()? {
                        compatible.push(s.to_owned());
                    }
                }
                "reg" => {
                    parse_reg(&mut reg, &prop, found_bus)?;
                }
                "interrupts" => {
                    let mut cell = Vec::new();
                    for i in 0.. {
                        if let Ok(value) = prop.u32(i) {
                            cell.push(value);
                        } else {
                            break;
                        }
                    }

                    let irq_matcher = match INTERRUPT_CONTROLLER.parse_interrupts_cell(&cell) {
                        Ok(irq) => irq,
                        Err(e) => {
                            panic!("{}: failed to parse interrupts cell: {:?}", node_name, e);
                        }
                    };

                    interrupts.push(irq_matcher);
                }
                "interrupt-parent" => {
                    let value = prop.phandle(0)?;
                    let Some(intc) = found_intcs.get_mut(&value) else {
                        panic!("{}: interrupt parent not found: {}", node_name, value);
                    };

                    if !intc.is_compatible {
                        warn!(
                            "{}: unsupported interrupt controller ({})",
                            node_name, intc.name
                        );
                    }
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
                interrupts,
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
