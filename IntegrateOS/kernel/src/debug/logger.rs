//! Structured Kernel Logging Engine
//!
//! Provides dynamic severity filtering, target subsystem tagging, dual-sink
//! console emission (VGA Text Console + COM1 Serial UART), and in-memory ring buffer recording.

use alloc::format;
use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::debug::manager::with_debug_mut;
use crate::debug::ring_buffer::LogLevel;
use crate::serial::SerialWriter;
use crate::vga::{self, Color, VgaWriter};
use core::fmt::Write;

static CURRENT_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Trace as u8);

/// Sets the maximum logging severity threshold. Messages below this level are ignored.
pub fn set_max_level(level: LogLevel) {
    CURRENT_LOG_LEVEL.store(level as u8, Ordering::Release);
}

/// Returns the currently active maximum logging level.
pub fn get_max_level() -> LogLevel {
    match CURRENT_LOG_LEVEL.load(Ordering::Acquire) {
        0 => LogLevel::Trace,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        _ => LogLevel::Fatal,
    }
}

/// Returns true if messages at `level` should be logged.
#[inline(always)]
pub fn is_level_enabled(level: LogLevel) -> bool {
    (level as u8) >= CURRENT_LOG_LEVEL.load(Ordering::Relaxed)
}

/// Core structured logging dispatch function.
#[doc(hidden)]
pub fn _log(level: LogLevel, target: &'static str, args: fmt::Arguments<'_>) {
    if !is_level_enabled(level) {
        return;
    }

    let msg = format!("{}", args);

    // 1. Color mapping for VGA console
    let (vga_fg, vga_bg) = match level {
        LogLevel::Trace => (Color::DarkGray, Color::Black),
        LogLevel::Debug => (Color::LightBlue, Color::Black),
        LogLevel::Info  => (Color::LightCyan, Color::Black),
        LogLevel::Warn  => (Color::Yellow, Color::Black),
        LogLevel::Error => (Color::LightRed, Color::Black),
        LogLevel::Fatal => (Color::White, Color::Red),
    };

    // 2. Emit to VGA Console
    vga::set_color(vga_fg, vga_bg);
    let _ = write!(VgaWriter, "[{}] [{}] {}\n", level.as_str(), target, msg);
    vga::set_color(Color::LightGray, Color::Black);

    // 3. Emit to Serial COM1
    let _ = write!(SerialWriter, "[{}] [{}] {}\n", level.as_str(), target, msg);

    // 4. Record into in-memory diagnostic ring buffer if DebugManager is initialized
    with_debug_mut(|dm| {
        dm.push_log(level, target, msg);
    });
}

#[macro_export]
macro_rules! klog_trace {
    ($target:expr, $($arg:tt)*) => {
        $crate::debug::logger::_log(
            $crate::debug::ring_buffer::LogLevel::Trace,
            $target,
            core::format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! klog_debug {
    ($target:expr, $($arg:tt)*) => {
        $crate::debug::logger::_log(
            $crate::debug::ring_buffer::LogLevel::Debug,
            $target,
            core::format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! klog_warn {
    ($target:expr, $($arg:tt)*) => {
        $crate::debug::logger::_log(
            $crate::debug::ring_buffer::LogLevel::Warn,
            $target,
            core::format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! klog_error {
    ($target:expr, $($arg:tt)*) => {
        $crate::debug::logger::_log(
            $crate::debug::ring_buffer::LogLevel::Error,
            $target,
            core::format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! klog_fatal {
    ($target:expr, $($arg:tt)*) => {
        $crate::debug::logger::_log(
            $crate::debug::ring_buffer::LogLevel::Fatal,
            $target,
            core::format_args!($($arg)*)
        )
    };
}
