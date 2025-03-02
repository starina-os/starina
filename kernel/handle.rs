use core::any::Any;
use core::ops::Deref;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::poll::Readiness;

use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::utils::FxHashMap;

const NUM_HANDLES_MAX: usize = 128;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: Handleable + ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

impl<T: Handleable + ?Sized> Handle<T> {
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
    pub fn new<T: Handleable>(h: Handle<T>) -> AnyHandle {
        Self(Handle {
            object: h.object, // upcasting happens here (thanks to CoerceUnsized)
            rights: h.rights,
        })
    }

    pub fn downcast<T: Handleable>(self) -> Option<Handle<T>> {
        let object = self.0.object.downcast().ok()?;
        let rights = self.0.rights;
        Some(Handle { object, rights })
    }
}

impl Deref for AnyHandle {
    type Target = dyn Handleable;

    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}
pub trait Handleable: Any + Send + Sync {
    fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode>;
    fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode>;
    fn readiness(&self) -> Result<Readiness, ErrorCode>;
}

pub struct HandleTable {
    handles: FxHashMap<HandleId, AnyHandle>,
    next_id: i32,
}

impl HandleTable {
    pub const fn new() -> HandleTable {
        HandleTable {
            handles: FxHashMap::new(),
            next_id: 1,
        }
    }

    pub fn insert(&mut self, object: AnyHandle) -> Result<HandleId, ErrorCode> {
        if self.handles.len() >= NUM_HANDLES_MAX {
            return Err(ErrorCode::TooManyHandles);
        }

        self.handles
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;

        let handle_id = HandleId::from_raw(self.next_id);
        self.handles.insert(handle_id, object);
        self.next_id += 1;
        Ok(handle_id)
    }

    pub fn is_movable(&self, handle: HandleId) -> bool {
        let exists = self.handles.get(&handle).is_some();
        exists
    }

    pub fn get<T: Handleable>(&self, handle: HandleId) -> Result<Handle<T>, ErrorCode> {
        let any_handle = self
            .handles
            .get(&handle)
            .cloned()
            .ok_or(ErrorCode::NotFound)?;

        let handle = any_handle.downcast().ok_or(ErrorCode::UnexpectedType)?;
        Ok(handle)
    }

    pub fn remove(&mut self, handle: HandleId) -> Option<AnyHandle> {
        self.handles.remove(&handle)
    }
}
