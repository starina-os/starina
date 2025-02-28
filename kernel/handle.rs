use alloc::collections::btree_map::BTreeMap;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;

use crate::channel::Channel;
use crate::poll::ListenerSet;
use crate::refcount::SharedRef;
use crate::spinlock::SpinLockGuard;

const NUM_HANDLES_MAX: usize = 128;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

pub enum AnyHandle {
    Channel(Handle<Channel>),
}

impl AnyHandle {
    pub fn listeners_mut(&self) -> SpinLockGuard<'_, ListenerSet> {
        todo!()
    }
}

pub struct HandleTable {
    handles: BTreeMap<HandleId, AnyHandle>,
    next_id: i32,
}

impl HandleTable {
    pub const fn new() -> HandleTable {
        HandleTable {
            handles: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn insert(&mut self, object: AnyHandle) -> Result<HandleId, ErrorCode> {
        if self.handles.len() >= NUM_HANDLES_MAX {
            return Err(ErrorCode::TooManyHandles);
        }

        let handle_id = HandleId::from_raw(self.next_id);
        self.handles.insert(handle_id, object);
        self.next_id += 1;
        Ok(handle_id)
    }

    pub fn is_movable(&self, handle: HandleId) -> bool {
        // Does the handle exist?
        self.handles.get(&handle).is_some()
    }

    pub fn get(&self, handle: HandleId) -> Option<&AnyHandle> {
        self.handles.get(&handle)
    }

    pub fn remove(&mut self, handle: HandleId) -> Option<AnyHandle> {
        self.handles.remove(&handle)
    }
}
