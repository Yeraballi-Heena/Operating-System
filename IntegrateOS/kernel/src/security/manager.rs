//! Central Security Manager & Per-Process Credential Registry
//!
//! Provides global security mediation, capability verification, resource quota accounting,
//! and audit logging for all kernel and user processes.

use alloc::collections::BTreeMap;

use crate::process::pid::ProcessId;
use crate::security::audit::{AuditEventKind, AuditLog, AuditSeverity};
use crate::security::types::{
    Capability, PointerViolation, ResourceLimitType, SecurityContext, SecurityError,
};
use crate::sync::IrqSpinlock;

/// Central security policy coordinator and process credentials table.
pub struct SecurityManager {
    contexts: BTreeMap<ProcessId, SecurityContext>,
    audit: AuditLog,
}

impl SecurityManager {
    /// Creates a new uninitialized security manager.
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            audit: AuditLog::new(),
        }
    }

    /// Registers a process's security context.
    pub fn register_process(&mut self, context: SecurityContext) {
        let pid = context.pid;
        let is_sandboxed = context.is_sandboxed;
        self.contexts.insert(pid, context);

        self.audit.record(
            pid,
            AuditSeverity::Info,
            AuditEventKind::SandboxEntered {
                profile_name: if is_sandboxed { "StrictSandbox" } else { "Standard" },
            },
            "Process security credentials registered",
        );
    }

    /// Unregisters and purges credentials for a terminated process.
    pub fn unregister_process(&mut self, pid: ProcessId) {
        self.contexts.remove(&pid);
    }

    /// Looks up the immutable security context of a process.
    pub fn get_context(&self, pid: ProcessId) -> Option<&SecurityContext> {
        self.contexts.get(&pid)
    }

    /// Looks up the mutable security context of a process.
    pub fn get_context_mut(&mut self, pid: ProcessId) -> Option<&mut SecurityContext> {
        self.contexts.get_mut(&pid)
    }

    /// Checks if a process holds the specified capability, logging an audit denial if missing.
    pub fn check_capability(
        &mut self,
        pid: ProcessId,
        cap: Capability,
        action: &'static str,
    ) -> Result<(), SecurityError> {
        let ctx = self.contexts.get(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        if ctx.has_capability(cap) {
            Ok(())
        } else {
            self.audit.record(
                pid,
                AuditSeverity::Denial,
                AuditEventKind::CapabilityDenied { cap, action },
                action,
            );
            Err(SecurityError::MissingCapability(cap))
        }
    }

    /// Permanently drops a capability from a process's active and bounding sets.
    pub fn drop_capability(
        &mut self,
        pid: ProcessId,
        cap: Capability,
    ) -> Result<(), SecurityError> {
        let ctx = self.contexts.get_mut(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        ctx.drop_capability(cap)?;
        let remaining = ctx.capabilities.bits();

        self.audit.record(
            pid,
            AuditSeverity::Info,
            AuditEventKind::PrivilegeDropped {
                dropped: cap,
                remaining_bits: remaining,
            },
            "Process dropped capability permanently",
        );
        Ok(())
    }

    /// Charges memory allocation against a process's memory quota.
    pub fn charge_memory(&mut self, pid: ProcessId, bytes: u64) -> Result<(), SecurityError> {
        let ctx = self.contexts.get_mut(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        match ctx.limits.alloc_memory(bytes) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.audit.record(
                    pid,
                    AuditSeverity::Denial,
                    AuditEventKind::ResourceExceeded {
                        limit: ResourceLimitType::MaxMemoryBytes,
                        requested: bytes,
                        current: ctx.limits.allocated_memory_bytes,
                        max: ctx.limits.max_memory_bytes,
                    },
                    "Process exceeded memory quota",
                );
                Err(e)
            }
        }
    }

    /// Releases memory allocation from a process's accounting.
    pub fn release_memory(&mut self, pid: ProcessId, bytes: u64) {
        if let Some(ctx) = self.contexts.get_mut(&pid) {
            ctx.limits.free_memory(bytes);
        }
    }

    /// Charges an open file against a process's file descriptor quota.
    pub fn charge_file(&mut self, pid: ProcessId) -> Result<(), SecurityError> {
        let ctx = self.contexts.get_mut(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        match ctx.limits.alloc_file() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.audit.record(
                    pid,
                    AuditSeverity::Denial,
                    AuditEventKind::ResourceExceeded {
                        limit: ResourceLimitType::MaxOpenFiles,
                        requested: 1,
                        current: ctx.limits.open_files_count as u64,
                        max: ctx.limits.max_open_files as u64,
                    },
                    "Process exceeded open file descriptor quota",
                );
                Err(e)
            }
        }
    }

    /// Releases an open file descriptor from a process's accounting.
    pub fn release_file(&mut self, pid: ProcessId) {
        if let Some(ctx) = self.contexts.get_mut(&pid) {
            ctx.limits.free_file();
        }
    }

    /// Charges child process creation against quota.
    pub fn charge_process(&mut self, pid: ProcessId) -> Result<(), SecurityError> {
        let ctx = self.contexts.get_mut(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        match ctx.limits.alloc_process() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.audit.record(
                    pid,
                    AuditSeverity::Denial,
                    AuditEventKind::ResourceExceeded {
                        limit: ResourceLimitType::MaxChildProcesses,
                        requested: 1,
                        current: ctx.limits.child_processes_count as u64,
                        max: ctx.limits.max_child_processes as u64,
                    },
                    "Process exceeded child processes quota",
                );
                Err(e)
            }
        }
    }

    /// Releases a terminated child process from a process's accounting.
    pub fn release_process(&mut self, pid: ProcessId) {
        if let Some(ctx) = self.contexts.get_mut(&pid) {
            ctx.limits.free_process();
        }
    }

    /// Charges execution time against CPU quota.
    pub fn consume_cpu_time(&mut self, pid: ProcessId, ms: u64) -> Result<(), SecurityError> {
        let ctx = self.contexts.get_mut(&pid).ok_or(SecurityError::ProcessNotFound(pid))?;
        match ctx.limits.consume_cpu(ms) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.audit.record(
                    pid,
                    AuditSeverity::Denial,
                    AuditEventKind::ResourceExceeded {
                        limit: ResourceLimitType::MaxCpuTimeMs,
                        requested: ms,
                        current: ctx.limits.consumed_cpu_time_ms,
                        max: ctx.limits.max_cpu_time_ms,
                    },
                    "Process exceeded CPU execution time quota",
                );
                Err(e)
            }
        }
    }

    /// Records an invalid user-space pointer or memory access fault in the audit log.
    pub fn record_pointer_fault(
        &mut self,
        pid: ProcessId,
        addr: u64,
        len: usize,
        violation: PointerViolation,
    ) {
        self.audit.record(
            pid,
            AuditSeverity::Fault,
            AuditEventKind::PointerFault { addr, len, violation },
            "User pointer validation fault",
        );
    }

    /// Records an unauthorized action attempt in the audit log.
    pub fn record_unauthorized_access(
        &mut self,
        pid: ProcessId,
        reason: &'static str,
    ) {
        self.audit.record(
            pid,
            AuditSeverity::Critical,
            AuditEventKind::UnauthorizedAccessAttempt { reason },
            reason,
        );
    }

    /// Access the audit log.
    #[inline(always)]
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit
    }

    /// Mutably access the audit log.
    #[inline(always)]
    pub fn audit_log_mut(&mut self) -> &mut AuditLog {
        &mut self.audit
    }

    /// Returns the number of registered security contexts.
    #[inline(always)]
    pub fn contexts_count(&self) -> usize {
        self.contexts.len()
    }
}

/// Global system security manager singleton protected by an interrupt-safe spinlock.
pub static SECURITY_MANAGER: IrqSpinlock<Option<SecurityManager>> = IrqSpinlock::new(None);

/// Executes a closure with an immutable reference to the central security manager.
pub fn with_security<R, F: FnOnce(&SecurityManager) -> R>(f: F) -> Option<R> {
    let guard = SECURITY_MANAGER.lock();
    guard.as_ref().map(f)
}

/// Executes a closure with a mutable reference to the central security manager.
pub fn with_security_mut<R, F: FnOnce(&mut SecurityManager) -> R>(f: F) -> Option<R> {
    let mut guard = SECURITY_MANAGER.lock();
    guard.as_mut().map(f)
}
