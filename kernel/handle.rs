use core::ops::Deref;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use hashbrown::HashMap;

use crate::app_loader::KernelAppMemory;
use crate::channel::Channel;
use crate::ref_counted::SharedRef;
use crate::thread::Thread;

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
pub struct Handle<T: ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

impl<T> Handle<T> {
    pub const fn new(object: SharedRef<T>, rights: HandleRights) -> Handle<T> {
        Handle { object, rights }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        Handle {
            object: self.object.clone(),
            rights: self.rights,
        }
    }
}

impl<T> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.object
    }
}

pub enum AnyHandle {
    Channel(Handle<Channel>),
    Thread(Handle<Thread>),
    KernelAppMemory(Handle<KernelAppMemory>),
}

impl AnyHandle {
    pub fn as_channel(&self) -> Result<&Handle<Channel>, FtlError> {
        match self {
            AnyHandle::Channel(ref channel) => Ok(channel),
            _ => Err(FtlError::UnexpectedHandleType),
        }
    }
}

impl Into<AnyHandle> for Handle<Channel> {
    fn into(self) -> AnyHandle {
        AnyHandle::Channel(self)
    }
}

impl Into<AnyHandle> for Handle<Thread> {
    fn into(self) -> AnyHandle {
        AnyHandle::Thread(self)
    }
}

impl Into<AnyHandle> for Handle<KernelAppMemory> {
    fn into(self) -> AnyHandle {
        AnyHandle::KernelAppMemory(self)
    }
}

/// The number of maximum handles per process.
///
/// The current 64K limit has no particular reason, but it should be low
/// enough to prevent an overflow in `next_id + 1` in `HandleTable::add`.
///
/// The hard limit is `2^HANDLE_ID_BITS - 1`.
const NUM_HANDLES_MAX: i32 = 64 * 1024 - 1;

pub struct HandleTable {
    next_id: i32,
    handles: HashMap<HandleId, AnyHandle>,
}

impl HandleTable {
    pub fn new() -> HandleTable {
        HandleTable {
            next_id: 1,
            handles: HashMap::new(),
        }
    }

    /// Add a handle to the table.
    pub fn add<H: Into<AnyHandle>>(&mut self, handle: H) -> Result<HandleId, FtlError> {
        if self.next_id >= NUM_HANDLES_MAX {
            return Err(FtlError::TooManyHandles);
        }

        let id = HandleId::from_raw(self.next_id);
        self.next_id = self.next_id + 1;
        self.handles.insert(id, handle.into());
        Ok(id)
    }

    /// Get a handle by ID.
    pub fn get_owned(&self, id: HandleId) -> Result<&AnyHandle, FtlError> {
        let handle = self.handles.get(&id).ok_or(FtlError::HandleNotFound)?;
        Ok(handle)
    }
}
