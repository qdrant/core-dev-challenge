use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub struct AtomicLockGuard<'atomic> {
    lock: &'atomic AtomicBool,
}

impl<'atomic> AtomicLockGuard<'atomic> {
    pub fn lock(lock: &'atomic AtomicBool) -> Self {
        Self::do_lock(lock);
        Self { lock }
    }

    fn do_lock(lock: &'atomic AtomicBool) {
        while lock.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl Drop for AtomicLockGuard<'_> {
    fn drop(&mut self) {
        self.unlock();
    }
}

pub(crate) struct AtomicF64(AtomicU64);

impl AtomicF64 {
    pub fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }

    pub fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.0.load(order))
    }

    pub fn store(&self, value: f64, order: Ordering) {
        self.0.store(value.to_bits(), order);
    }

    pub fn swap(&self, value: f64, order: Ordering) -> f64 {
        f64::from_bits(self.0.swap(value.to_bits(), order))
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

        let old_value = atomic_f64.swap(42, Ordering::Relaxed);
        assert_eq!(old_value, 2.71);

        assert_eq!(atomic_f64.load(Ordering::Relaxed), 42);
    }
}
