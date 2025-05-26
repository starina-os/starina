use alloc::collections::btree_map::BTreeMap;
use core::any::Any;
use core::ops::Deref;

use starina_types::error::ErrorCode;
use starina_types::handle::HandleId;
use starina_types::handle::HandleRights;
use starina_types::poll::Readiness;

use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::SharedRef;

const NUM_HANDLES_MAX: usize = 128;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: Handleable + ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

impl<T: Handleable + ?Sized> Handle<T> {
    pub fn new(object: SharedRef<T>, rights: HandleRights) -> Handle<T> {
        Handle { object, rights }
    }

    pub fn into_object(self) -> SharedRef<T> {
        self.object
    }

    pub fn is_capable(&self, required: HandleRights) -> bool {
        self.rights.is_capable(required)
    }
}

impl<T: Handleable + ?Sized> Deref for Handle<T> {
    type Target = SharedRef<T>;

    fn deref(&self) -> &SharedRef<T> {
        &self.object
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

impl AnyHandle {
    pub fn downcast<T: Handleable>(self) -> Option<Handle<T>> {
        let object = self.0.object.downcast().ok()?;
        let rights = self.0.rights;
        Some(Handle { object, rights })
    }
}

impl<T: Handleable> From<Handle<T>> for AnyHandle {
    fn from(h: Handle<T>) -> AnyHandle {
        Self(Handle {
            object: h.object, // upcasting happens here (thanks to CoerceUnsized)
            rights: h.rights,
        })
    }
}

impl Deref for AnyHandle {
    type Target = dyn Handleable;

    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}
pub trait Handleable: Any + Send + Sync {
    fn close(&self);
    fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode>;
    fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode>;
    fn readiness(&self) -> Result<Readiness, ErrorCode>;
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

    pub fn insert<H: Into<AnyHandle>>(&mut self, object: H) -> Result<HandleId, ErrorCode> {
        if self.handles.len() >= NUM_HANDLES_MAX {
            return Err(ErrorCode::TooManyHandles);
        }

        let handle_id = HandleId::from_raw(self.next_id);
        let value = object.into();

        if self.handles.try_insert(handle_id, value).is_err() {
            return Err(ErrorCode::AlreadyExists);
        }

        self.next_id += 1;
        Ok(handle_id)
    }

    /// Insert two handles, and return the first ID.
    ///
    /// The IDs are guaranteed to be consecutive.
    pub fn insert_consecutive<H: Into<AnyHandle>>(
        &mut self,
        first: H,
        second: H,
    ) -> Result<HandleId, ErrorCode> {
        if self.handles.len() + 2 > NUM_HANDLES_MAX {
            return Err(ErrorCode::TooManyHandles);
        }

        let first_id = HandleId::from_raw(self.next_id);
        let second_id = HandleId::from_raw(self.next_id + 1);

        if self.handles.contains_key(&first_id) || self.handles.contains_key(&second_id) {
            return Err(ErrorCode::AlreadyExists);
        }

        self.handles.insert(first_id, first.into());
        self.handles.insert(second_id, second.into());

        self.next_id += 2;
        Ok(first_id)
    }

    pub fn is_movable(&self, handle: HandleId) -> bool {
        let exists = self.handles.get(&handle).is_some();
        exists
    }

    pub fn get_any(&self, handle: HandleId) -> Result<AnyHandle, ErrorCode> {
        self.handles
            .get(&handle)
            .cloned()
            .ok_or(ErrorCode::NotFound)
    }

    pub fn get<T: Handleable>(&self, handle: HandleId) -> Result<Handle<T>, ErrorCode> {
        let any_handle = self.get_any(handle)?;
        let handle = any_handle.downcast().ok_or(ErrorCode::UnexpectedType)?;
        Ok(handle)
    }

    pub fn take(&mut self, handle: HandleId) -> Option<AnyHandle> {
        self.handles.remove(&handle)
    }

    pub fn close(&mut self, handle: HandleId) -> Result<(), ErrorCode> {
        let handle = self.handles.remove(&handle).ok_or(ErrorCode::NotFound)?;
        handle.close();
        Ok(())
    }
}
