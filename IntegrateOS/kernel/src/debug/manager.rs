//! Central Kernel Debug Coordinator & Telemetry Manager
//!
//! Manages in-memory diagnostic telemetry, crash statistics, log ring buffer,
//! and post-mortem inspection.

use alloc::string::String;

use crate::debug::ring_buffer::{LogLevel, LogRingBuffer};
use crate::sync::spinlock::IrqSpinlock;

/// Central kernel debugging state and diagnostic telemetry.
pub struct DebugManager {
    ring_buffer: LogRingBuffer,
    fault_count: usize,
    panic_count: usize,
}

impl DebugManager {
    /// Creates a new debug manager with empty ring buffer and zero fault counts.
    pub fn new() -> Self {
        Self {
            ring_buffer: LogRingBuffer::new(),
            fault_count: 0,
            panic_count: 0,
        }
    }

    /// Records a structured log entry into the circular buffer.
    pub fn push_log(&mut self, level: LogLevel, target: &'static str, message: String) {
        self.ring_buffer.push(level, target, message);
    }

    /// Returns an immutable reference to the diagnostic log ring buffer.
    #[inline(always)]
    pub fn ring_buffer(&self) -> &LogRingBuffer {
        &self.ring_buffer
    }

    /// Returns a mutable reference to the diagnostic log ring buffer.
    #[inline(always)]
    pub fn ring_buffer_mut(&mut self) -> &mut LogRingBuffer {
        &mut self.ring_buffer
    }

    /// Increments the total CPU/memory fault counter.
    pub fn record_fault(&mut self) {
        self.fault_count += 1;
    }

    /// Returns the total number of recorded faults.
    #[inline(always)]
    pub fn fault_count(&self) -> usize {
        self.fault_count
    }

    /// Increments the panic counter.
    pub fn record_panic(&mut self) {
        self.panic_count += 1;
    }

    /// Returns the total number of recorded panics.
    #[inline(always)]
    pub fn panic_count(&self) -> usize {
        self.panic_count
    }
}

/// Global system debug coordinator protected by an interrupt-safe spinlock.
pub static DEBUG_MANAGER: IrqSpinlock<Option<DebugManager>> = IrqSpinlock::new(None);

/// Executes a closure with an immutable reference to the central debug manager.
pub fn with_debug<R, F: FnOnce(&DebugManager) -> R>(f: F) -> Option<R> {
    let guard = DEBUG_MANAGER.lock();
    guard.as_ref().map(f)
}

/// Executes a closure with a mutable reference to the central debug manager.
pub fn with_debug_mut<R, F: FnOnce(&mut DebugManager) -> R>(f: F) -> Option<R> {
    let mut guard = DEBUG_MANAGER.lock();
    guard.as_mut().map(f)
}
