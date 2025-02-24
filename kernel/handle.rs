use alloc::collections::btree_map::BTreeMap;

use starina::handle::HandleId;
use starina::handle::HandleRights;

use crate::refcount::SharedRef;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

pub enum AnyHandle {}

pub struct HandleTable {
    handles: BTreeMap<HandleId, AnyHandle>,
}

impl HandleTable {
    pub const fn new() -> HandleTable {
        HandleTable {
            handles: BTreeMap::new(),
        }
    }
}
