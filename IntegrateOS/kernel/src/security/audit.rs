//! Security Audit Trail & Event Logging Subsystem
//!
//! Captures and maintains an immutable structured log of security events:
//! capability denials, user pointer violations, resource quota exhaustion, and privilege drops.

use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::x86_64::pit;
use crate::process::pid::ProcessId;
use crate::security::types::{Capability, PointerViolation, ResourceLimitType};

/// Severity classification for security audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    /// Informational event (e.g. process registered, capability dropped).
    Info,
    /// Low-risk warning event.
    Warning,
    /// Capability or permission denial (attempt blocked).
    Denial,
    /// Invalid memory pointer or bad address fault.
    Fault,
    /// Critical security violation or sandbox breach attempt.
    Critical,
}

impl AuditSeverity {
    pub const fn name(&self) -> &'static str {
        match self {
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARN",
            AuditSeverity::Denial => "DENIAL",
            AuditSeverity::Fault => "FAULT",
            AuditSeverity::Critical => "CRITICAL",
        }
    }
}

/// Category and metadata of a security audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventKind {
    /// User memory pointer failed bounds or page table checks.
    PointerFault {
        addr: u64,
        len: usize,
        violation: PointerViolation,
    },
    /// Attempt to exercise an unheld capability.
    CapabilityDenied {
        cap: Capability,
        action: &'static str,
    },
    /// Process exceeded its allocated resource quota.
    ResourceExceeded {
        limit: ResourceLimitType,
        requested: u64,
        current: u64,
        max: u64,
    },
    /// Process dropped a capability from its bounding set.
    PrivilegeDropped {
        dropped: Capability,
        remaining_bits: u32,
    },
    /// Process entered an isolated sandbox.
    SandboxEntered {
        profile_name: &'static str,
    },
    /// Generic unauthorized action attempt.
    UnauthorizedAccessAttempt {
        reason: &'static str,
    },
}

/// A structured security audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub seq: u64,
    pub timestamp_ticks: u64,
    pub pid: ProcessId,
    pub severity: AuditSeverity,
    pub kind: AuditEventKind,
    pub description: String,
}

/// In-memory ring buffer holding recent security audit events.
pub const AUDIT_CAPACITY: usize = 128;

pub struct AuditLog {
    events: Vec<AuditEvent>,
    next_seq: u64,
    total_denials: usize,
    total_faults: usize,
}

impl AuditLog {
    /// Creates a new audit log.
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(AUDIT_CAPACITY),
            next_seq: 1,
            total_denials: 0,
            total_faults: 0,
        }
    }

    /// Records a new security audit event into the ring buffer.
    pub fn record(
        &mut self,
        pid: ProcessId,
        severity: AuditSeverity,
        kind: AuditEventKind,
        desc: &str,
    ) {
        if severity == AuditSeverity::Denial {
            self.total_denials += 1;
        } else if severity == AuditSeverity::Fault || severity == AuditSeverity::Critical {
            self.total_faults += 1;
        }

        if self.events.len() >= AUDIT_CAPACITY {
            self.events.remove(0); // Evict oldest event
        }

        let ticks = pit::get_ticks();
        let event = AuditEvent {
            seq: self.next_seq,
            timestamp_ticks: ticks,
            pid,
            severity,
            kind,
            description: String::from(desc),
        };
        self.next_seq += 1;
        self.events.push(event);
    }

    /// Total count of permission and capability denials recorded.
    #[inline(always)]
    pub fn total_denials(&self) -> usize {
        self.total_denials
    }

    /// Total count of memory pointer faults recorded.
    #[inline(always)]
    pub fn total_faults(&self) -> usize {
        self.total_faults
    }

    /// Total number of events currently stored in the log.
    #[inline(always)]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Slice of all recorded audit events.
    #[inline(always)]
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Returns the most recently logged audit event.
    #[inline(always)]
    pub fn last_event(&self) -> Option<&AuditEvent> {
        self.events.last()
    }

    /// Clears the event buffer (retains monotonic sequence counter).
    pub fn clear(&mut self) {
        self.events.clear();
    }
}
