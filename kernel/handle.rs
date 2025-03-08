use core::any::Any;
use core::ops::Deref;

use starina::error::ErrorCode;
use starina::handle::HandleId;
use starina::handle::HandleRights;
use starina::poll::Readiness;

use crate::channel::Channel;
use crate::poll::Listener;
use crate::poll::Poll;
use crate::refcount::SharedRef;
use crate::utils::fxhashmap::FxHashMap;

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
pub enum AnyHandle {
    Channel(Handle<Channel>),
    Poll(Handle<Poll>),
}

impl AnyHandle {
    pub fn into_channel(self) -> Option<Handle<Channel>> {
        match self {
            AnyHandle::Channel(h) => Some(h),
            _ => None,
        }
    }

    pub fn into_poll(self) -> Option<Handle<Poll>> {
        match self {
            AnyHandle::Poll(h) => Some(h),
            _ => None,
        }
    }

    pub fn close(&self) {
        match self {
            AnyHandle::Channel(h) => h.close(),
            AnyHandle::Poll(h) => h.close(),
        }
    }

    pub fn add_listener(&self, listener: Listener) -> Result<(), ErrorCode> {
        match self {
            AnyHandle::Channel(h) => h.add_listener(listener),
            AnyHandle::Poll(h) => h.add_listener(listener),
        }
    }

    pub fn remove_listener(&self, poll: &Poll) -> Result<(), ErrorCode> {
        match self {
            AnyHandle::Channel(h) => h.remove_listener(poll),
            AnyHandle::Poll(h) => h.remove_listener(poll),
        }
    }

    pub fn readiness(&self) -> Result<Readiness, ErrorCode> {
        match self {
            AnyHandle::Channel(h) => h.readiness(),
            AnyHandle::Poll(h) => h.readiness(),
        }
    }
}

impl From<Handle<Channel>> for AnyHandle {
    fn from(h: Handle<Channel>) -> Self {
        AnyHandle::Channel(h)
    }
}

impl From<Handle<Poll>> for AnyHandle {
    fn from(h: Handle<Poll>) -> Self {
        AnyHandle::Poll(h)
    }
}

pub trait Handleable: Any + Send + Sync {
    fn close(&self);
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

    pub fn insert<H: Into<AnyHandle>>(&mut self, object: H) -> Result<HandleId, ErrorCode> {
        if self.handles.len() >= NUM_HANDLES_MAX {
            return Err(ErrorCode::TooManyHandles);
        }

        self.handles
            .try_reserve(1)
            .map_err(|_| ErrorCode::OutOfMemory)?;

        let handle_id = HandleId::from_raw(self.next_id);
        self.handles.insert(handle_id, object.into());
        self.next_id += 1;
        Ok(handle_id)
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

    pub fn get_as_channel(&self, handle: HandleId) -> Result<Handle<Channel>, ErrorCode> {
        let any_handle = self.get_any(handle)?;
        let handle = any_handle.into_channel().ok_or(ErrorCode::UnexpectedType)?;
        Ok(handle)
    }

    pub fn get_as_poll(&self, handle: HandleId) -> Result<Handle<Poll>, ErrorCode> {
        let any_handle = self.get_any(handle)?;
        let handle = any_handle.into_poll().ok_or(ErrorCode::UnexpectedType)?;
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
