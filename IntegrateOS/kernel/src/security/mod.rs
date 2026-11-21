//! Integrate OS — Security & Isolation Architecture (Phase 30)
//!
//! Provides:
//! - Hardware-enforced and MMU-validated user pointer security (`pointer.rs`)
//! - Fine-grained capability-based access control with one-way revocation (`types.rs`)
//! - Strict multi-tier process isolation and sandboxing (`sandbox.rs`)
//! - Process resource limits, accounting, and quota enforcement (`types.rs`, `manager.rs`)
//! - Central security coordinator and credentials registry (`manager.rs`)
//! - Structured security audit logging trail (`audit.rs`)
//! - Verification test suite (`tests.rs`)

pub mod audit;
pub mod manager;
pub mod pointer;
pub mod sandbox;
pub mod tests;
pub mod types;

use core::sync::atomic::{AtomicBool, Ordering};

pub use audit::{AuditEvent, AuditEventKind, AuditLog, AuditSeverity};
pub use manager::{with_security, with_security_mut, SecurityManager, SECURITY_MANAGER};
pub use pointer::{validate_user_address, UserPtr, UserSlice};
pub use sandbox::SandboxPolicy;
pub use types::{
    Capability, CapabilitySet, PointerViolation, ResourceLimitType, ResourceLimits,
    SecurityContext, SecurityError,
};

use crate::klog_ok;
use crate::process::pid::ProcessId;

static SECURITY_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns true if the security subsystem has been initialized.
#[inline(always)]
pub fn is_security_initialized() -> bool {
    SECURITY_INITIALIZED.load(Ordering::Acquire)
}

/// Initializes the system security manager, root credentials, and audit logging.
pub fn init() {
    let mut sm = SecurityManager::new();

    // Register bootstrap kernel process PID 0 with full supervisor capabilities
    let root_context = SecurityContext::new_root(ProcessId::KERNEL, "kernel_supervisor");
    sm.register_process(root_context);

    *SECURITY_MANAGER.lock() = Some(sm);
    SECURITY_INITIALIZED.store(true, Ordering::Release);

    klog_ok!("Security & Isolation Subsystem Initialized: capabilities, sandboxing, and audit active");
}
