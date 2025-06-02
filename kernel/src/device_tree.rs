use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::slice;

use fdt_rs::base::*;
use fdt_rs::index::DevTreeIndex;
use fdt_rs::index::DevTreeIndexNode;
use fdt_rs::index::DevTreeIndexProp;
use fdt_rs::prelude::*;
use fdt_rs::spec::fdt_header;
use hashbrown::HashMap;
use starina::address::PAddr;
use starina::device_tree::DeviceNode;
use starina::device_tree::DeviceTree;
use starina::device_tree::Reg;
use starina_utils::byte_size::ByteSize;

use crate::allocator::GLOBAL_ALLOCATOR;
use crate::arch::INTERRUPT_CONTROLLER;
use crate::arch::find_free_ram;
use crate::arch::paddr2vaddr;

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

struct RegParser<'a, 'i, 'dt> {
    address_cells: u32,
    size_cells: u32,
    reg: DevTreeIndexProp<'a, 'i, 'dt>,
    next_index: usize,
}

impl<'a, 'i, 'dt> RegParser<'a, 'i, 'dt> {
    pub fn parse(
        node: DevTreeIndexNode<'a, 'i, 'dt>,
    ) -> Result<Option<RegParser<'a, 'i, 'dt>>, fdt_rs::error::DevTreeError> {
        let mut reg = None;
        for prop in node.props() {
            if prop.name()? == "reg" {
                reg = Some(prop);
            }
        }

        let Some(reg) = reg else {
            return Ok(None);
        };

        let mut current = Some(node);
        while let Some(n) = current {
            let mut address_cells = None;
            let mut size_cells = None;
            for prop in n.props() {
                let prop_name = prop.name()?;
                match prop_name {
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

            if let (Some(address_cells), Some(size_cells)) = (address_cells, size_cells) {
                return Ok(Some(RegParser {
                    reg,
                    address_cells,
                    size_cells,
                    next_index: 0,
                }));
            }

            current = n.parent();
        }

        Ok(None)
    }
}

fn extract_nth_u32(reg: &DevTreeIndexProp<'_, '_, '_>, index: usize) -> u64 {
    reg.u32(index).unwrap() as u64
}

impl<'a, 'i, 'dt> Iterator for RegParser<'a, 'i, 'dt> {
    type Item = Reg;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reg.u32(self.next_index).is_err() {
            // End of the regs.
            return None;
        }

        let addr = match self.address_cells {
            1 => extract_nth_u32(&self.reg, self.next_index),
            2 => {
                let high = extract_nth_u32(&self.reg, self.next_index);
                let low = extract_nth_u32(&self.reg, self.next_index + 1);
                (high << 32) | low
            }
            _ => {
                panic!("unsupported address cells: {}", self.address_cells);
            }
        };
        self.next_index += self.address_cells as usize;

        let size = match self.size_cells {
            1 => extract_nth_u32(&self.reg, self.next_index),
            2 => {
                let high = extract_nth_u32(&self.reg, self.next_index);
                let low = extract_nth_u32(&self.reg, self.next_index + 1);
                (high << 32) | low
            }
            _ => {
                panic!("unsupported size cells: {}", self.size_cells);
            }
        };

        self.next_index += self.size_cells as usize;
        Some(Reg { addr, size })
    }
}

#[derive(Debug)]
struct FoundBus {
    is_referenced: bool,
    address_cells: u32,
    size_cells: u32,
}

#[derive(Debug)]
struct FoundInterruptController {
    name: String,
    is_compatible: bool,
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

    for node in devtree_index.nodes() {
        let mut device_type = None;
        for prop in node.props() {
            let prop_name = prop.name()?;
            if prop_name == "device_type" {
                device_type = Some(prop.str()?);
            }
        }

        let Some(device_type) = device_type else {
            continue;
        };

        if device_type == "memory" {
            let iter = RegParser::parse(node)?.expect("missing reg for a memory node");
            for reg in iter {
                let addr: usize = reg.addr.try_into().unwrap();
                let unchecked_size = reg.size.try_into().unwrap();
                let paddr = PAddr::new(addr);
                find_free_ram(paddr, unchecked_size, |paddr, size| {
                    match paddr2vaddr(paddr) {
                        Ok(vaddr) => {
                            debug!("free RAM: vaddr={} ({})", vaddr, ByteSize(size));
                            let ptr = unsafe { vaddr.as_mut_ptr() };
                            GLOBAL_ALLOCATOR.add_region(ptr, size);
                        }
                        Err(_) => {
                            debug_warn!(
                                "unmappable memory node at {} (size: {}), ignoring",
                                paddr,
                                unchecked_size
                            );
                        }
                    }
                });
            }
        }
    }

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

        if compatible.is_empty() {
            continue;
        }

        let is_compatible = INTERRUPT_CONTROLLER.try_init(&compatible, &reg).is_ok();
        found_intcs.insert(
            phandle,
            FoundInterruptController {
                name: node_name.to_owned(),
                is_compatible,
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
                reg,
                interrupts,
            },
        );
    }

    Ok(DeviceTree { devices })
}
