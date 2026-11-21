//! One-Time Initialization Primitives (`Once` & `OnceCell`)
//!
//! Guarantees that critical kernel setup or lazy initialization routines run exactly
//! once in a thread-safe manner without race conditions.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

const INCOMPLETE: usize = 0;
const RUNNING: usize = 1;
const COMPLETE: usize = 2;

/// A synchronization primitive used to run a one-time initialization routine.
pub struct Once {
    state: AtomicUsize,
}

impl Once {
    /// Creates a new uncompleted `Once` flag.
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(INCOMPLETE),
        }
    }

    /// Performs an initialization routine exactly once.
    /// If another task is concurrently executing the routine, callers yield/spin until complete.
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if self.state.load(Ordering::Acquire) == COMPLETE {
            return;
        }

        if self
            .state
            .compare_exchange(INCOMPLETE, RUNNING, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            f();
            self.state.store(COMPLETE, Ordering::Release);
        } else {
            // Another thread is running or finished
            while self.state.load(Ordering::Acquire) != COMPLETE {
                crate::scheduler::yield_now();
            }
        }
    }

    /// Returns true if the initialization routine has completed.
    #[inline(always)]
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == COMPLETE
    }
}

/// A cell which can be written to only once, providing safe shared access.
pub struct OnceCell<T> {
    once: Once,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync + Send> Sync for OnceCell<T> {}
unsafe impl<T: Send> Send for OnceCell<T> {}

impl<T> OnceCell<T> {
    /// Creates a new empty `OnceCell`.
    pub const fn new() -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(None),
        }
    }

    /// Attempts to set the value. Returns `Err(value)` if already initialized.
    pub fn set(&self, val: T) -> Result<(), T> {
        let mut result = Ok(());
        let mut slot = Some(val);

        self.once.call_once(|| {
            // SAFETY: We are inside call_once, exclusive execution.
            unsafe {
                *self.value.get() = slot.take();
            }
        });

        if let Some(rejected) = slot {
            result = Err(rejected);
        }

        result
    }

    /// Gets a reference to the contained value if initialized.
    pub fn get(&self) -> Option<&T> {
        if self.once.is_completed() {
            // SAFETY: Value is initialized and will not be mutated again.
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }

    /// Gets the value or initializes it with the closure `f`.
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        self.once.call_once(|| {
            let val = f();
            unsafe {
                *self.value.get() = Some(val);
            }
        });
        // SAFETY: Value is guaranteed to be set now.
        unsafe { (*self.value.get()).as_ref().unwrap() }
    }
}
