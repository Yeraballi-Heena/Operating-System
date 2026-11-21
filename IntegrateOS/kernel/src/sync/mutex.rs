//! Kernel Sleeping / Cooperative Mutex (`KernelMutex`)
//!
//! Unlike a pure spinlock that burns CPU cycles indefinitely during lock contention,
//! a `KernelMutex` cooperatively yields execution via the scheduler when waiting.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A mutual exclusion primitive that cooperatively yields when contested.
pub struct KernelMutex<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for KernelMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for KernelMutex<T> {}

/// RAII lock guard for `KernelMutex`.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    pub(crate) mutex: &'a KernelMutex<T>,
    data: &'a mut T,
}

impl<T> KernelMutex<T> {
    /// Creates a new unlocked `KernelMutex`.
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the mutex, yielding the CPU to other ready processes if contested.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Cooperatively yield the current timeslice to another runnable task
            crate::scheduler::yield_now();
        }

        MutexGuard {
            mutex: self,
            // SAFETY: Mutex is held exclusively.
            data: unsafe { &mut *self.data.get() },
        }
    }

    /// Attempts to acquire the mutex without yielding or blocking.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(MutexGuard {
                mutex: self,
                // SAFETY: Mutex is held exclusively.
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    /// Returns true if the mutex is currently locked.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Returns a mutable reference to the underlying data.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}
