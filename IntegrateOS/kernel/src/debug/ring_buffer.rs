//! In-Memory Kernel Diagnostic Ring Buffer
//!
//! Stores the most recent structured kernel log messages in an in-memory buffer,
//! facilitating post-mortem analysis and crash dump reporting.

use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::x86_64::pit;

/// Logging severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info  => "INFO ",
            Self::Warn  => "WARN ",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

/// A structured log entry stored in the diagnostic ring buffer.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp_ticks: u64,
    pub level: LogLevel,
    pub target: &'static str,
    pub message: String,
}

/// Capacity of the circular diagnostic log buffer.
pub const LOG_RING_BUFFER_CAPACITY: usize = 256;

/// Circular log buffer for kernel debugging and crash inspection.
pub struct LogRingBuffer {
    entries: Vec<LogEntry>,
    next_seq: u64,
}

impl LogRingBuffer {
    /// Creates a new empty log ring buffer.
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(LOG_RING_BUFFER_CAPACITY),
            next_seq: 1,
        }
    }

    /// Records a new log entry, evicting the oldest entry when capacity is reached.
    pub fn push(&mut self, level: LogLevel, target: &'static str, message: String) {
        if self.entries.len() >= LOG_RING_BUFFER_CAPACITY {
            self.entries.remove(0);
        }

        let ticks = pit::get_ticks();
        let entry = LogEntry {
            seq: self.next_seq,
            timestamp_ticks: ticks,
            level,
            target,
            message,
        };
        self.next_seq += 1;
        self.entries.push(entry);
    }

    /// Returns the total number of entries currently stored in the ring buffer.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a slice of all stored log entries in chronological order.
    #[inline(always)]
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Returns up to `count` of the most recent log entries.
    pub fn recent(&self, count: usize) -> &[LogEntry] {
        let n = self.entries.len();
        if count >= n {
            &self.entries
        } else {
            &self.entries[n - count..]
        }
    }

    /// Clears the ring buffer.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
