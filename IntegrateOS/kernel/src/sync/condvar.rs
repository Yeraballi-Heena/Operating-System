//! Condition Variable (`Condvar`)
//!
//! Enables tasks to wait for a shared condition to become true while releasing
//! an associated mutex.

use crate::sync::mutex::{KernelMutex, MutexGuard};
use crate::sync::wait_queue::WaitQueue;

/// A condition variable for task notification and synchronization.
pub struct Condvar {
    wait_queue: WaitQueue,
}

impl Condvar {
    /// Creates a new condition variable.
    pub const fn new() -> Self {
        Self {
            wait_queue: WaitQueue::new(),
        }
    }

    /// Atomically releases the mutex and blocks the current task until notified.
    /// Re-acquires the mutex before returning.
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        let mutex: &'a KernelMutex<T> = guard.mutex;
        // Drop the guard to release the mutex
        drop(guard);

        // Put current task on wait queue and block
        self.wait_queue.wait_current();

        // Re-acquire the mutex upon wakeup
        mutex.lock()
    }

    /// Wakes up one waiting task.
    pub fn notify_one(&self) -> bool {
        self.wait_queue.wake_one().is_some()
    }

    /// Wakes up all waiting tasks.
    pub fn notify_all(&self) -> usize {
        self.wait_queue.wake_all()
    }

    /// Returns the number of tasks currently waiting on this condition variable.
    #[inline(always)]
    pub fn waiter_count(&self) -> usize {
        self.wait_queue.len()
    }
}
