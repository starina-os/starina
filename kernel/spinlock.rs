use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// A simple spinlock.
///
/// # Interrupts must be disabled!
///
/// *Before* acquiring the lock, interrupts must be disabled. This is because
/// the lock may be used in interrupt context, which easily leads to deadlocks.
///
/// Typically, in-kernel spinlock provides a way to disable and re-enable
/// interrupts (e.g. Linux's `spin_lock_irqsave`), but our kernel assumes
/// that kernel is not preemptive (i.e. interrupts are disabled). Therefore,
/// it is the caller's responsibility to disable interrupts before acquiring
/// the lock.
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
            panic!(
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

impl<T> SpinLockGuard<'_, T> {
    pub fn lock(&self) -> &SpinLock<T> {
        &self.this
    }
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

unsafe impl<T: ?Sized + Sync> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

unsafe impl<T: ?Sized + Sync> Sync for SpinLockGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLockGuard<'_, T> {}
