//! Spinlock & Hardware Interrupt Critical Section Primitives
//!
//! Provides:
//! - `IrqSpinlock<T>`: Spinlock safe against concurrent interrupts on the local core.
//! - `Spinlock<T>`: Standard spinlock for non-interrupt contexts.
//! - `without_interrupts`: Helper to execute closures within an interrupt-disabled critical section.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// Disables interrupts on the local processor and returns whether interrupts were previously enabled.
#[inline(always)]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = push_and_disable_interrupts();
    let ret = f();
    restore_interrupts(was_enabled);
    ret
}

/// Disables interrupts and returns true if interrupts were enabled before calling `cli`.
#[inline(always)]
pub fn push_and_disable_interrupts() -> bool {
    let rflags: u64;
    // SAFETY: Reading RFLAGS and disabling interrupts (cli).
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            "cli",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }
    (rflags & (1 << 9)) != 0
}

/// Restores interrupts if `was_enabled` is true.
#[inline(always)]
pub fn restore_interrupts(was_enabled: bool) {
    if was_enabled {
        // SAFETY: Re-enabling interrupts if they were previously enabled.
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
        }
    }
}

/// A mutual exclusion lock safe to use inside and outside interrupt contexts.
pub struct IrqSpinlock<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for IrqSpinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for IrqSpinlock<T> {}

/// An RAII guard holding an acquired `IrqSpinlock` with interrupts disabled.
pub struct IrqSpinlockGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
    was_interrupt_enabled: bool,
}

impl<T> IrqSpinlock<T> {
    /// Creates a new unlocked `IrqSpinlock`.
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, spinning until available while disabling interrupts.
    pub fn lock(&self) -> IrqSpinlockGuard<'_, T> {
        let was_enabled = push_and_disable_interrupts();
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }

        IrqSpinlockGuard {
            lock: &self.lock,
            // SAFETY: We hold the spinlock exclusively and interrupts are disabled.
            data: unsafe { &mut *self.data.get() },
            was_interrupt_enabled: was_enabled,
        }
    }

    /// Attempts to acquire the lock without spinning.
    pub fn try_lock(&self) -> Option<IrqSpinlockGuard<'_, T>> {
        let was_enabled = push_and_disable_interrupts();
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(IrqSpinlockGuard {
                lock: &self.lock,
                // SAFETY: Lock acquired.
                data: unsafe { &mut *self.data.get() },
                was_interrupt_enabled: was_enabled,
            })
        } else {
            restore_interrupts(was_enabled);
            None
        }
    }

    /// Returns true if the lock is currently held.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    /// Returns a mutable reference to the underlying data.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<'a, T: ?Sized> Deref for IrqSpinlockGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for IrqSpinlockGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for IrqSpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        restore_interrupts(self.was_interrupt_enabled);
    }
}

/// Standard Spinlock (does not disable interrupts).
pub struct Spinlock<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

/// An RAII guard holding an acquired standard `Spinlock`.
pub struct SpinlockGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Spinlock<T> {
    /// Creates a new unlocked `Spinlock`.
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, spinning until available.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }

        SpinlockGuard {
            lock: &self.lock,
            // SAFETY: Lock held exclusively.
            data: unsafe { &mut *self.data.get() },
        }
    }

    /// Attempts to acquire the lock without spinning.
    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinlockGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    /// Returns true if the lock is currently held.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    /// Returns a mutable reference to the underlying data.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<'a, T: ?Sized> Deref for SpinlockGuard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SpinlockGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}
