use std::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

/// A lock guard for an atomic boolean lock.
///
/// The lock is locked when the boolean is `true` and unlocked when it is `false`.
///
/// Dropping the `AtomicLockGuard` will automatically unlock the lock.
pub struct AtomicLockGuard<'atomic> {
    lock: &'atomic AtomicBool,
}

impl<'atomic> AtomicLockGuard<'atomic> {
    /// Creates a new `AtomicLockGuard` by locking the provided atomic boolean.
    ///
    /// It will wait indefinitely until the lock can be acquired.
    #[inline]
    pub fn lock(lock: &'atomic AtomicBool) -> Self {
        Self::do_lock(lock);
        Self { lock }
    }

    fn do_lock(lock: &'atomic AtomicBool) {
        while lock.swap(true, Ordering::Acquire) {
            spin_loop();
        }
    }

    #[inline]
    fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl Drop for AtomicLockGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        self.unlock();
    }
}

/// An atomic floating-point type that provides atomic load and store-related operations on `f64` values.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct AtomicF64(AtomicU64);

impl AtomicF64 {
    #[inline]
    pub fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.0.load(order))
    }

    #[inline]
    pub fn store(&self, value: f64, order: Ordering) {
        self.0.store(value.to_bits(), order);
    }

}

pub(crate) struct AtomicSemaphoreGuard<'atomic> {
    lock: &'atomic AtomicU64,
    drop_order: Ordering,
}

impl<'atomic> AtomicSemaphoreGuard<'atomic> {
    #[inline]
    pub fn increment(lock: &'atomic AtomicU64, order: Ordering, drop_order: Ordering) -> Self {
        lock.fetch_add(1, order);
        Self { lock, drop_order }
    }
}

impl<'atomic> Drop for AtomicSemaphoreGuard<'atomic> {
    #[inline]
    fn drop(&mut self) {
        self.lock.fetch_sub(1, self.drop_order);
    }
}
pub struct AtomicMutex<T> {
    value: UnsafeCell<T>,
    atomlock: AtomicBool,
}

unsafe impl<T: Send> Send for AtomicMutex<T> {}

unsafe impl<T: Send> Sync for AtomicMutex<T> {}

pub struct AtomicMutexGuard<'a, T> {
    parent: &'a AtomicMutex<T>,
}

impl<T> AtomicMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::<T>::new(value),
            atomlock: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> AtomicMutexGuard<'_, T> {
        while self.atomlock.swap(true, Ordering::AcqRel) {
            spin_loop()
        }
        AtomicMutexGuard { parent: self }
    }

    pub fn try_lock(&self) -> Option<AtomicMutexGuard<'_, T>> {
        if !self.atomlock.swap(true, Ordering::AcqRel) {
            Some(AtomicMutexGuard { parent: self })
        } else {
            None
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for AtomicMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.lock();
        f.debug_struct("AtomicMutex")
            .field("value", &*guard)
            .finish()
    }
}

impl<T> Drop for AtomicMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.parent.atomlock.store(false, Ordering::Release);
    }
}

impl<T> Deref for AtomicMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.parent.value.get() }
    }
}

impl<T> DerefMut for AtomicMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.value.get() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_lock_smoke() {
        let lock = std::sync::atomic::AtomicBool::new(false);
        {
            let _guard = AtomicLockGuard::lock(&lock);
            assert!(lock.load(std::sync::atomic::Ordering::Relaxed));
        }
        assert!(!lock.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_atomic_f64_smoke() {
        let atomic_f64 = AtomicF64::new(3.14);
        assert_eq!(atomic_f64.load(Ordering::Relaxed), 3.14);

        atomic_f64.store(2.71, Ordering::Relaxed);
        assert_eq!(atomic_f64.load(Ordering::Relaxed), 2.71);
    }

    #[test]
    fn test_atomic_semaphore_guard_smoke() {
        let lock = AtomicU64::new(0);
        {
            let _guard1 =
                AtomicSemaphoreGuard::increment(&lock, Ordering::Relaxed, Ordering::Relaxed);
            assert_eq!(lock.load(Ordering::Relaxed), 1);
            {
                let _guard2 =
                    AtomicSemaphoreGuard::increment(&lock, Ordering::Relaxed, Ordering::Relaxed);
                assert_eq!(lock.load(Ordering::Relaxed), 2);
            }
            assert_eq!(lock.load(Ordering::Relaxed), 1);
        }
        assert_eq!(lock.load(Ordering::Relaxed), 0);
    }
}
