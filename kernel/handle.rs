use alloc::collections::btree_map::BTreeMap;
use core::ops::Deref;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::poll::Readiness;

use crate::poll::Listener;
use crate::refcount::SharedRef;

const NUM_HANDLES_MAX: usize = 128;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: Handleable + ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

impl<T: Handleable + ?Sized> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.object
    }
}

impl<T: Handleable + ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            object: self.object.clone(),
            rights: self.rights,
        }
    }
}

#[derive(Clone)]
pub struct AnyHandle(Handle<dyn Handleable>);

impl Deref for AnyHandle {
    type Target = dyn Handleable;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
pub trait Handleable: Send + Sync {
    fn add_listener(&self, listener: SharedRef<Listener>);
    fn readiness(&self) -> Readiness;
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
