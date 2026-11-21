//! Reader-Writer Lock (`RwLock`)
//!
//! Provides concurrent read access for multiple readers, or exclusive write access
//! for a single writer.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicIsize, Ordering};

const UNLOCKED: isize = 0;
const WRITE_LOCKED: isize = -1;

/// A reader-writer lock allowing multiple concurrent readers or one exclusive writer.
pub struct RwLock<T: ?Sized> {
    state: AtomicIsize,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}
unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}

/// RAII read guard for `RwLock`.
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

/// RAII write guard for `RwLock`.
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<T> RwLock<T> {
    /// Creates a new unlocked `RwLock`.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicIsize::new(UNLOCKED),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires shared read access, spinning until available.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let current = self.state.load(Ordering::Relaxed);
            if current >= 0 {
                if self
                    .state
                    .compare_exchange_weak(
                        current,
                        current + 1,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return RwLockReadGuard { lock: self };
                }
            } else {
                core::hint::spin_loop();
            }
        }
    }

    /// Attempts to acquire shared read access without spinning.
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let current = self.state.load(Ordering::Relaxed);
        if current >= 0
            && self
                .state
                .compare_exchange(current, current + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        {
            Some(RwLockReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Acquires exclusive write access, spinning until all readers and writers have exited.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        loop {
            if self
                .state
                .compare_exchange_weak(UNLOCKED, WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return RwLockWriteGuard { lock: self };
            }
            while self.state.load(Ordering::Relaxed) != UNLOCKED {
                core::hint::spin_loop();
            }
        }
    }

    /// Attempts to acquire exclusive write access without spinning.
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(UNLOCKED, WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(RwLockWriteGuard { lock: self })
        } else {
            None
        }
    }

    /// Returns the number of currently active readers (0 if unlocked or write-locked).
    #[inline(always)]
    pub fn reader_count(&self) -> usize {
        let s = self.state.load(Ordering::Relaxed);
        if s > 0 {
            s as usize
        } else {
            0
        }
    }

    /// Returns true if currently held by a writer.
    #[inline(always)]
    pub fn is_write_locked(&self) -> bool {
        self.state.load(Ordering::Relaxed) == WRITE_LOCKED
    }

    /// Returns a mutable reference to the underlying data.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: Held with shared read lock (state > 0).
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: Held with exclusive write lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Held with exclusive write lock.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.store(UNLOCKED, Ordering::Release);
    }
}
