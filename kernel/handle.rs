use core::fmt;
use core::ops::Deref;

use ftl_types::error::FtlError;
use ftl_types::handle::HandleId;
use ftl_types::handle::HandleRights;
use ftl_types::handle::HANDLE_ID_MASK;
use hashbrown::HashMap;

use crate::buffer::Buffer;
use crate::channel::Channel;
use crate::poll::Poll;
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
    Buffer(Handle<Buffer>),
    Poll(Handle<Poll>),
}

impl AnyHandle {
    pub fn as_channel(&self) -> Result<&Handle<Channel>, FtlError> {
        match self {
            AnyHandle::Channel(ref channel) => Ok(channel),
            _ => Err(FtlError::UnexpectedHandleType),
        }
    }

    pub fn as_poll(&self) -> Result<&Handle<Poll>, FtlError> {
        match self {
            AnyHandle::Poll(ref poll) => Ok(poll),
            _ => Err(FtlError::UnexpectedHandleType),
        }
    }
}

impl fmt::Debug for AnyHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Implement Debug for each handle type.
        match self {
            AnyHandle::Channel(_) => write!(f, "Channel"),
            AnyHandle::Thread(_) => write!(f, "Thread"),
            AnyHandle::Buffer(_) => write!(f, "Buffer"),
            AnyHandle::Poll(_) => write!(f, "Poll"),
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

impl Into<AnyHandle> for Handle<Buffer> {
    fn into(self) -> AnyHandle {
        AnyHandle::Buffer(self)
    }
}

impl Into<AnyHandle> for Handle<Poll> {
    fn into(self) -> AnyHandle {
        AnyHandle::Poll(self)
    }
}

/// The number of maximum handles per process.
///
/// The current 64K limit has no particular reason, but it should be low
/// enough to prevent a process from consuming too many resources.
const NUM_HANDLES_MAX: usize = 64 * 1024;

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

    pub fn is_movable(&self, id: HandleId) -> bool {
        self.handles.get(&id).is_some()
    }

    /// Add a handle to the table.
    pub fn add<H: Into<AnyHandle>>(&mut self, handle: H) -> Result<HandleId, FtlError> {
        if self.handles.len() >= NUM_HANDLES_MAX {
            return Err(FtlError::TooManyHandles);
        }

        if self.next_id == 0 {
            self.next_id += 1;
        }

        let id = HandleId::from_raw(self.next_id);
        self.next_id = (self.next_id + 1) & HANDLE_ID_MASK;

        debug_assert!(id.as_i32() != 0);
        self.handles.insert(id, handle.into());
        Ok(id)
    }

    /// Get a handle by ID.
    pub fn get_owned(&self, id: HandleId) -> Result<&AnyHandle, FtlError> {
        let handle = self.handles.get(&id).ok_or(FtlError::HandleNotFound)?;
        Ok(handle)
    }

    /// Removes a handle out of the table.
    pub fn remove(&mut self, id: HandleId) -> Result<AnyHandle, FtlError> {
        let handle = self.handles.remove(&id).ok_or(FtlError::HandleNotFound)?;
        Ok(handle)
    }
}
