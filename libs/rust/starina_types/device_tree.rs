use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::slice;

use fdt_rs::base::*;
use fdt_rs::prelude::*;
use fdt_rs::spec::fdt_header;
use hashbrown::HashMap;
use serde::Deserialize;
use serde::Serialize;

/// The device tree. This is the root of the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceTree {
    pub nodes: HashMap<String, DeviceNode>,
}

/// A node in the device tree.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceNode {
    pub compatible: Option<Vec<String>>,
    pub reg: Option<Vec<(u64, u64)>>,
}

#[derive(Debug)]
pub enum ParseError {
    InvalidMagicNumber(fdt_rs::error::DevTreeError),
    InvalidSize(fdt_rs::error::DevTreeError),
    InvalidTree(fdt_rs::error::DevTreeError),
    InvalidNode(fdt_rs::error::DevTreeError),
    InvalidName(fdt_rs::error::DevTreeError),
    InvalidProp(fdt_rs::error::DevTreeError),
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

        let mut node_iter = devtree.nodes();
        let mut root_nodes = HashMap::new();
        while let Some(node) = node_iter.next().map_err(ParseError::InvalidNode)? {
            let node_name = node.name().map_err(ParseError::InvalidName)?;
            let mut prop_iter = node.props();
            let mut compatible = None;
            let mut reg = None;
            while let Some(prop) = prop_iter.next().map_err(ParseError::InvalidProp)? {}

            root_nodes.insert(node_name.to_owned(), DeviceNode { compatible, reg });
        }

        Ok(DeviceTree { nodes: root_nodes })
    }
}
