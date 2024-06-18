use core::any::Any;
use core::num::NonZeroIsize;
use core::ops::Deref;

use ftl_types::error::FtlError;
use ftl_types::handle::{HandleId, HandleRights};
use ftl_utils::downcast::Downcastable;
use hashbrown::HashMap;

use crate::ref_counted::SharedRef;

/// A trait for kernel objects that can be referred to by a handle ([`Handle`]).
pub trait Handleable: Any + Sync + Send {}

/// Handle, a reference-counted pointer to a kernel object with allowed
/// operations on it, aka *"capability"*.
///
/// # Reference Counting
///
/// This type uses some atomic operations to keep track of the number of
/// references to the underlying object. [`Ordering`] parameters are chosen
/// to be as relaxed as possible in the fast path, inspired by Rust's `Arc`
/// implementation.
pub struct Handle<T: Handleable + ?Sized> {
    object: SharedRef<T>,
    rights: HandleRights,
}

impl<T: Handleable> Handle<T> {
    pub fn rights(&self) -> HandleRights {
        self.rights
    }
}

impl<T: Handleable> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.object
    }
}

unsafe impl<T: Handleable + ?Sized> Sync for Handle<T> {}
unsafe impl<T: Handleable + ?Sized> Send for Handle<T> {}

pub struct AnyHandle(Handle<dyn Handleable>);

impl AnyHandle {
    pub fn new<T: Handleable>(object: SharedRef<T>, rights: HandleRights) -> AnyHandle {
        AnyHandle(Handle {
            object: object as SharedRef<dyn Handleable>,
            rights: rights,
        })
    }

    pub fn downcast<T: Handleable>(&self) -> Option<&Handle<T>> {
        self.0.as_any().downcast_ref::<Handle<T>>()
    }
}

/// The number of maximum handles per process.
///
/// The current 64K limit has no particular reason, but it should be low
/// enough to prevent an overflow in `next_id + 1` in `HandleTable::add`.
const NUM_HANDLES_MAX: isize = 64 * 1024;

pub struct HandleTable {
    next_id: isize,
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
    pub fn add(&mut self, handle: AnyHandle) -> Result<HandleId, FtlError> {
        if self.next_id >= NUM_HANDLES_MAX {
            return Err(FtlError::TooManyHandles);
        }

        // SAFETY: The condition above ensures it doesn't overflow and
        //         never reaches zero.
        let raw_id = unsafe { NonZeroIsize::new_unchecked(self.next_id) };
        let id = HandleId::from_nonzero(raw_id);

        self.next_id = self.next_id + 1;
        self.handles.insert(id, handle);
        Ok(id)
    }

    /// Get a handle by ID, as a concrete type `T`.
    pub fn get<T>(&self, id: HandleId) -> Result<&Handle<T>, FtlError>
    where
        T: Handleable,
    {
        let any_handle = self.handles.get(&id).ok_or(FtlError::HandleNotFound)?;
        let handle = any_handle
            .downcast()
            .ok_or(FtlError::UnexpectedHandleType)?;
        Ok(handle)
    }
}
