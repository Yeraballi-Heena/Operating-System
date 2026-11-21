//! Integrate OS — Time & Real-Time Clock (RTC) Subsystem Architecture
//!
//! Provides hardware CMOS RTC interrogation, Gregorian calendar calculations,
//! ISO-8601 formatting, Unix timestamp conversions, strictly monotonic uptime tracking,
//! and the kernel software timer service.

pub mod boot_profiler;
pub mod clock;
pub mod cmos;
pub mod manager;
pub mod timer;
pub mod tests;
pub mod types;

pub use boot_profiler::{log_boot_comparison_table, record_component_boot, record_stage, BOOT_PROFILER};
pub use clock::{MonotonicClock, WallClock};
pub use cmos::CmosRtc;
pub use manager::{
    format_now_iso8601, now, timestamp, uptime_ms, with_time_mut, TimeManager, TIME_MANAGER,
};
pub use timer::{TimerEntry, TimerId, TimerMode, TimerService};
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static TIME_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the time subsystem has initialized.
pub fn is_time_initialized() -> bool {
    TIME_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the time subsystem, probes CMOS RTC, and configures the wall clock.
pub fn init() {
    let mut mgr = TimeManager::new();
    let result = mgr.init();

    if let Err(e) = result {
        crate::klog_info!("[time] Hardware RTC init notice: {}", e.as_str());
    }

    *TIME_MANAGER.lock() = Some(mgr);
    TIME_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[time] Time Subsystem initialized");
}
