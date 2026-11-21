//! Integrate OS — System Monotonic & Wall-Clock Engines
//!
//! Provides strictly monotonic uptime tracking derived from PIT ticks
//! and synchronized real-world civil time (Wall-Clock) anchored to the CMOS RTC.

use crate::arch::x86_64::pit;
use core::sync::atomic::{AtomicU64, Ordering};
use super::cmos::CmosRtc;
use super::types::{DateTime, Duration};

/// System Monotonic Uptime Clock.
pub struct MonotonicClock;

impl MonotonicClock {
    /// Returns the raw hardware PIT ticks elapsed since system startup.
    #[inline]
    pub fn uptime_ticks() -> u64 {
        pit::get_ticks()
    }

    /// Returns the system uptime in milliseconds (10 ms per PIT tick at 100 Hz).
    #[inline]
    pub fn uptime_millis() -> u64 {
        Self::uptime_ticks() * 10
    }

    /// Returns the system uptime in microseconds.
    #[inline]
    pub fn uptime_micros() -> u64 {
        Self::uptime_millis() * 1000
    }

    /// Returns the system uptime in whole seconds.
    #[inline]
    pub fn uptime_seconds() -> u64 {
        Self::uptime_ticks() / 100
    }

    /// Returns the total elapsed uptime as a `Duration`.
    #[inline]
    pub fn uptime_duration() -> Duration {
        Duration::from_millis(Self::uptime_millis())
    }
}

/// Global anchor: Unix timestamp in seconds at system boot.
static BOOT_REALTIME_SECS: AtomicU64 = AtomicU64::new(0);

/// System Wall-Clock (Real-World Civil Time).
pub struct WallClock;

impl WallClock {
    /// Synchronizes the wall clock baseline with the motherboard CMOS RTC.
    pub fn sync_with_rtc() {
        if let Ok(rtc_dt) = CmosRtc::read_datetime() {
            let epoch_secs = rtc_dt.to_unix_timestamp();
            let cur_uptime_secs = MonotonicClock::uptime_seconds();
            // Subtract current uptime to anchor to boot time
            let boot_anchor = epoch_secs.saturating_sub(cur_uptime_secs);
            BOOT_REALTIME_SECS.store(boot_anchor, Ordering::SeqCst);
        } else {
            // Fallback to static 2026-11-15 epoch if RTC read fails
            let fallback_dt = DateTime::new(2026, 11, 15, 12, 0, 0);
            BOOT_REALTIME_SECS.store(fallback_dt.to_unix_timestamp(), Ordering::SeqCst);
        }
    }

    /// Explicitly updates the system wall-clock date and time.
    pub fn set_datetime(dt: DateTime) {
        let epoch_secs = dt.to_unix_timestamp();
        let cur_uptime_secs = MonotonicClock::uptime_seconds();
        let new_anchor = epoch_secs.saturating_sub(cur_uptime_secs);
        BOOT_REALTIME_SECS.store(new_anchor, Ordering::SeqCst);
    }

    /// Returns the current real-world Unix timestamp (seconds since 1970-01-01 00:00:00 UTC).
    #[inline]
    pub fn now_timestamp() -> u64 {
        let anchor = BOOT_REALTIME_SECS.load(Ordering::Relaxed);
        anchor + MonotonicClock::uptime_seconds()
    }

    /// Returns the current real-world date and time as a `DateTime` struct.
    #[inline]
    pub fn now() -> DateTime {
        DateTime::from_unix_timestamp(Self::now_timestamp())
    }
}
