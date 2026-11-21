//! Integrate OS — Debugging & Reliability Architecture (Phase 32)
//!
//! Provides:
//! - Structured serial logger with dynamic severity threshold filtering (`logger.rs`)
//! - In-memory diagnostic ring buffer for post-mortem analysis (`ring_buffer.rs`)
//! - Architectural x86_64 CPU exception error code decoding (`exception.rs`)
//! - Stack frame pointer chain unwinding, register readouts & hex dumping (`panic.rs`)
//! - Unified kernel error taxonomy with standard POSIX errno mapping (`errors.rs`)
//! - Central debug coordinator and crash telemetry manager (`manager.rs`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod errors;
pub mod exception;
pub mod logger;
pub mod manager;
pub mod panic;
pub mod ring_buffer;
pub mod tests;

use core::sync::atomic::{AtomicBool, Ordering};

pub use errors::{KernelErrorCategory, KernelErrorCode};
pub use exception::{exception_name, GeneralProtectionError, PageFaultError, TableIndicator};
pub use logger::{get_max_level, is_level_enabled, set_max_level};
pub use manager::{with_debug, with_debug_mut, DebugManager, DEBUG_MANAGER};
pub use panic::{format_hex_dump, read_rbp, read_rsp, unwind_stack, StackFrame};
pub use ring_buffer::{LogEntry, LogLevel, LogRingBuffer, LOG_RING_BUFFER_CAPACITY};

use crate::klog_ok;

static DEBUG_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns true if the debugging and reliability subsystem has been initialized.
#[inline(always)]
pub fn is_debug_initialized() -> bool {
    DEBUG_INITIALIZED.load(Ordering::Acquire)
}

/// Initializes the central debug manager, diagnostic ring buffer, and telemetry coordinator.
pub fn init() {
    let dm = DebugManager::new();
    *DEBUG_MANAGER.lock() = Some(dm);
    DEBUG_INITIALIZED.store(true, Ordering::Release);

    klog_ok!("Debugging & Reliability Subsystem Initialized: structured logger, ring buffer, exception decoder active");
}
