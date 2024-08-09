//! Reference counting.

use alloc::boxed::Box;
use core::any::Any;
use core::mem;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

struct RefCounted<T: ?Sized> {
    counter: AtomicUsize,
    value: T,
}

/// A reference-counted object.
///
/// # Why not `Arc`?
///
/// Rust's standard library provides `Arc` for reference counting. However, we
/// generally prefer rolling our own pritimives in kernel to use what we really
/// need.
///
/// In reference counting, we have some properties:
///
/// - We'll never need weak references. Instead, the userland will delete each
///   object explicitly through a system call (LMK if you noticed counter-examples!).
///
/// # Atomic Operations on counters
///
/// [`Ordering`] parameters are chosen to be as relaxed as possible in the fast
/// path, inspired by Rust's `Arc` implementation.
pub struct SharedRef<T: ?Sized> {
    ptr: NonNull<RefCounted<T>>,
}

impl<T> SharedRef<T> {
    /// Creates a new reference-counted object.
    pub fn new(value: T) -> Self {
        let ptr = Box::leak(Box::new(RefCounted {
            counter: AtomicUsize::new(1),
            value,
        }));

        Self {
            // SAFETY: Box always returns a valid non-null pointer.
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    pub fn ptr_eq(a: &SharedRef<T>, b: &SharedRef<T>) -> bool {
        a.ptr == b.ptr
    }
}

impl<T: ?Sized> SharedRef<T> {
    /// Returns a reference to the inner object.
    fn inner(&self) -> &RefCounted<T> {
        // SAFETY: The object will be kept alive as long as `self` is alive.
        //         The compiler will guarantee `&RefCounted<T>` can't outlive
        //         `self`.
        unsafe { self.ptr.as_ref() }
    }
}

impl SharedRef<dyn Any + Sync + Send> {
    pub fn downcast<T>(self) -> Result<SharedRef<T>, Self>
    where
        T: Any + Sync + Send,
    {
        if <dyn Any>::is::<T>(&self) {
            Ok(SharedRef {
                ptr: self.ptr.cast(),
            })
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized> Drop for SharedRef<T> {
    fn drop(&mut self) {
        // Release the reference count.
        if self.inner().counter.fetch_sub(1, Ordering::Release) == 1 {
            // The reference counter reached zero. Free the memory.

            // "Prevent reordering of use of the data and deletion of the data",
            // as the standard library's `Arc` does [1].
            //
            // [1]: https://github.com/rust-lang/rust/blob/da159eb331b27df528185c616b394bb0e1d2a4bd/library/alloc/src/sync.rs#L2469-L2497
            atomic::fence(Ordering::Acquire);

            // SAFETY: This reference was the last one, so we can safely
            //         free the memory.
            mem::drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
        }
    }
}

impl<T> Clone for SharedRef<T> {
    fn clone(&self) -> Self {
        // Increment the reference count.
        //
        // Theoretically, the counter can overflow, but it's not a problem
        // in practice because having 2^B references (where B is 32 or 64
        // depending on the CPU) means you have at least 2^B * size_of(NonNull)
        // bytes of space. Who would have that much memory to store references
        // to only single object?
        //
        // If you don't agree with this, please open a PR with a nice
        // explanation. It must be fun to read :)
        self.inner().counter.fetch_add(1, Ordering::Relaxed);

        Self { ptr: self.ptr }
    }
}

impl<T> Deref for SharedRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner().value
    }
}

unsafe impl<T: Sync> Sync for SharedRef<T> {}
unsafe impl<T: Send> Send for SharedRef<T> {}

pub struct StaticRef<T> {
    ref_counted: RefCounted<T>,
}

impl<T> StaticRef<T> {
    pub const fn new(value: T) -> Self {
        Self {
            ref_counted: RefCounted {
                // We initialize the counter to 1 as we do in SharedRef::new,
                // but we don't decrement it in the Drop. This means this object
                // will never be freed.
                counter: AtomicUsize::new(1),
                value,
            },
        }
    }

    pub fn shared_ref(&'static self) -> SharedRef<T> {
        self.ref_counted.counter.fetch_add(1, Ordering::Relaxed);

        SharedRef {
            ptr: NonNull::from(&self.ref_counted),
        }
    }
}

impl<T> Deref for StaticRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ref_counted.value
    }
}

impl<T> Drop for StaticRef<T> {
    fn drop(&mut self) {
        unreachable!("StaticSharedRef must not be dropped");
    }
}
