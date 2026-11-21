//! Integrate OS — Application Framework Core Types & Lifecycle States
//!
//! Provides instance identifiers, application execution states, and error types.

use core::sync::atomic::{AtomicU32, Ordering};

static NEXT_INSTANCE_ID: AtomicU32 = AtomicU32::new(1);

/// Strongly typed monotonic identifier for running application instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(pub u32);

impl InstanceId {
    /// Generates a new unique monotonic instance ID.
    pub fn next() -> Self {
        Self(NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Execution lifecycle state of an application instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Uninitialized,
    Running,
    Suspended,
    Terminated,
}

/// Errors occurring during application lifecycle operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppError {
    InitializationFailed,
    WindowCreationFailed,
    InstanceNotFound,
    AlreadyTerminated,
    ResourceExhausted,
}
