use alloc::vec::Vec;
use core::mem::size_of;
use core::slice;

use fdt_rs::base::*;
use fdt_rs::prelude::*;
use fdt_rs::spec::fdt_header;
use ftl_inlinedvec::InlinedVec;

pub struct Device {
    pub name: &'static str,
    pub compatible: &'static str,
    pub reg: u64,
    pub interrupts: Option<InlinedVec<u32, 2>>,
}

pub fn walk_device_nodes(dtb_addr: *const u8) -> Vec<Device> {
    let devtree = unsafe {
        // Check  the magic number and read the size of the device tree.
        let dtb_magic = { slice::from_raw_parts(dtb_addr, size_of::<fdt_header>()) };
        let size = DevTree::read_totalsize(dtb_magic).expect("failed to read device tree size");

        // Parse the device tree.
        let dtb = { slice::from_raw_parts(dtb_addr, size) };
        DevTree::new(dtb).expect("failed to load device tree")
    };

    let mut devices = Vec::new();
    let mut node_iter = devtree.nodes();
    while let Ok(Some(node)) = node_iter.next() {
        let mut prop_iter = node.props();
        let mut compatible = None;
        let mut reg = None;
        let mut interrupts = None;
        while let Ok(Some(prop)) = prop_iter.next() {
            match prop.name() {
                Ok("compatible") => {
                    compatible = prop.str().ok();
                }
                Ok("reg") => {
                    reg = prop.u64(0).ok();
                }
                Ok("interrupts") => {
                    if prop.length() > 0 {
                        let mut list = InlinedVec::new();
                        for i in 0..prop.length() {
                            if let Ok(interrupt) = prop.u32(i) {
                                list.try_push(interrupt).expect("too many interrupts");
                            }
                        }

                        interrupts = Some(list);
                    }
                }
                _ => {}
            }
        }

        if let (Some(compatible), Some(reg)) = (compatible, reg) {
            devices.push(Device {
                name: node.name().unwrap(),
                compatible,
                reg,
                interrupts,
            });
        }
    }

    devices
}
