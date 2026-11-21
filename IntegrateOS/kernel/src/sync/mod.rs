//! Integrate OS — Kernel Synchronization Subsystem (Phase 31)
//!
//! Provides:
//! - Spinlocks & Interrupt-Safe Critical Sections (`spinlock.rs`)
//! - Reader-Writer Locks with concurrent readers (`rwlock.rs`)
//! - Sleeping/Cooperative Kernel Mutexes (`mutex.rs`)
//! - Counting Semaphores for permit-based concurrency (`semaphore.rs`)
//! - Task Wait Queues for blocked thread management (`wait_queue.rs`)
//! - Condition Variables for thread notification (`condvar.rs`)
//! - Thread-Safe One-Time Initialization (`once.rs`)
//! - Thread Rendezvous Barriers (`barrier.rs`)
//! - Verification test suite (`tests.rs`)

pub mod barrier;
pub mod condvar;
pub mod mutex;
pub mod once;
pub mod rwlock;
pub mod semaphore;
pub mod spinlock;
pub mod tests;
pub mod wait_queue;

use core::sync::atomic::{AtomicBool, Ordering};

pub use barrier::{Barrier, BarrierWaitResult};
pub use condvar::Condvar;
pub use mutex::{KernelMutex, MutexGuard};
pub use once::{Once, OnceCell};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use semaphore::CountingSemaphore;
pub use spinlock::{
    push_and_disable_interrupts, restore_interrupts, without_interrupts, IrqSpinlock,
    IrqSpinlockGuard, Spinlock, SpinlockGuard,
};
pub use wait_queue::WaitQueue;

use crate::klog_ok;

static SYNC_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the kernel synchronization subsystem.
pub fn init() {
    SYNC_INITIALIZED.store(true, Ordering::Release);
    klog_ok!("Kernel Synchronization Subsystem Initialized: spinlocks, rwlocks, mutexes, and wait queues active");
}

/// Returns true if the synchronization subsystem has been initialized.
#[inline(always)]
pub fn is_sync_initialized() -> bool {
    SYNC_INITIALIZED.load(Ordering::Acquire)
}
