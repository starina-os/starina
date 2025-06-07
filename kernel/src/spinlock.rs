//! Spinlock implementation.
use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;
use core::panic::Location;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// A simple spinlock.
pub struct SpinLock<T: ?Sized> {
    lock: AtomicBool,
    locked_by: UnsafeCell<Option<Location<'static>>>,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            value: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
            locked_by: UnsafeCell::new(None),
        }
    }

    #[track_caller]
    pub fn lock(&self) -> SpinLockGuard<T> {
        if self.lock.load(Ordering::Relaxed) {
            println!(
                "spinlock: {:x}: deadlock detected - mutex will never be left locked in single CPU!\ncalled from: {}\ncurrently locked by: {:?}",
                self as *const _ as usize,
                Location::caller(),
                self.locked_by.get(),
            );
        }

        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        unsafe {
            *self.locked_by.get() = Some(*Location::caller());
        }
        SpinLockGuard { this: self }
    }
}

pub struct SpinLockGuard<'a, T: ?Sized + 'a> {
    this: &'a SpinLock<T>,
}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            *self.this.locked_by.get() = None;
        }

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
