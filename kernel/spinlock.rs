//! Spinlock implementation.
use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// A simple spinlock.
pub struct SpinLock<T: ?Sized> {
    lock: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            value: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        if self.lock.load(Ordering::Relaxed) {
            oops!(
                "spinlock: {:x}: deadlock detected - mutex will never be left locked in single CPU!",
                self as *const _ as usize
            );
        }

        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        SpinLockGuard { this: self }
    }
}

pub struct SpinLockGuard<'a, T: ?Sized + 'a> {
    this: &'a SpinLock<T>,
}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.this.lock.store(false, Ordering::Release);
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: The dereference is safe because this lock guard has
        // exclusive access to the data and the pointer is always valid.
        unsafe { &*self.this.value.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The dereference is safe because this lock guard has
        // exclusive access to the data and the pointer is always valid.
        unsafe { &mut *self.this.value.get() }
    }
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

unsafe impl<T: ?Sized + Sync> Sync for SpinLockGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLockGuard<'_, T> {}
