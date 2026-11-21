//! Counting Semaphore
//!
//! Provides thread-safe permit-based resource synchronization and bounded concurrency limits.

use core::sync::atomic::{AtomicIsize, Ordering};

/// A counting semaphore for bounded resource allocation.
pub struct CountingSemaphore {
    permits: AtomicIsize,
    max_permits: usize,
}

impl CountingSemaphore {
    /// Creates a new counting semaphore with an initial and maximum permit count.
    pub const fn new(initial: usize, max: usize) -> Self {
        Self {
            permits: AtomicIsize::new(initial as isize),
            max_permits: max,
        }
    }

    /// Acquires a permit from the semaphore, yielding execution if none are currently available.
    pub fn acquire(&self) {
        loop {
            let current = self.permits.load(Ordering::Relaxed);
            if current > 0 {
                if self
                    .permits
                    .compare_exchange_weak(current, current - 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
            } else {
                crate::scheduler::yield_now();
            }
        }
    }

    /// Attempts to acquire a permit without yielding.
    pub fn try_acquire(&self) -> bool {
        loop {
            let current = self.permits.load(Ordering::Relaxed);
            if current <= 0 {
                return false;
            }
            if self
                .permits
                .compare_exchange_weak(current, current - 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Releases a permit back to the semaphore, up to the maximum permit threshold.
    /// Returns true if permit was added, false if already at maximum.
    pub fn release(&self) -> bool {
        loop {
            let current = self.permits.load(Ordering::Relaxed);
            if current >= self.max_permits as isize {
                return false;
            }
            if self
                .permits
                .compare_exchange_weak(current, current + 1, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Returns the number of currently available permits.
    #[inline(always)]
    pub fn available_permits(&self) -> usize {
        let count = self.permits.load(Ordering::Relaxed);
        if count > 0 {
            count as usize
        } else {
            0
        }
    }

    /// Returns the maximum capacity of this semaphore.
    #[inline(always)]
    pub fn max_permits(&self) -> usize {
        self.max_permits
    }
}
