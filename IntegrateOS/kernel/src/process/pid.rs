//! Process Identifier (PID) Management
//!
//! Provides `ProcessId` and atomic monotonic allocation for unique process IDs.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u64);

impl ProcessId {
    /// PID 0 represents the kernel bootstrap / idle task.
    pub const KERNEL: Self = Self(0);

    /// PID 1 represents the initial system service manager.
    pub const INIT: Self = Self(1);

    /// Returns the raw integer value of this PID.
    #[inline(always)]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

static NEXT_PID: AtomicU64 = AtomicU64::new(0);

/// Allocates the next unique monotonic `ProcessId`.
pub fn next_pid() -> ProcessId {
    ProcessId(NEXT_PID.fetch_add(1, Ordering::Relaxed))
}

/// Resets the PID allocator (used during kernel initialization).
pub fn reset_pid_allocator(initial: u64) {
    NEXT_PID.store(initial, Ordering::Release);
}
