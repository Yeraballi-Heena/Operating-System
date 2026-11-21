//! Integrate OS — Central Time Subsystem Manager
//!
//! Orchestrates the CMOS Real-Time Clock, monotonic uptime counter,
//! wall-clock synchronization, and the global software timer service.

use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

use super::clock::{MonotonicClock, WallClock};
use super::cmos::CmosRtc;
use super::timer::{TimerId, TimerMode, TimerService};
use super::types::{DateTime, TimeError};

pub struct TimeManager {
    timers: TimerService,
    initial_rtc_dt: Option<DateTime>,
    battery_ok: bool,
}

impl TimeManager {
    pub const fn new() -> Self {
        Self {
            timers: TimerService::new(),
            initial_rtc_dt: None,
            battery_ok: false,
        }
    }

    /// Initializes the Time subsystem: queries CMOS RTC, syncs the wall-clock, and enables timers.
    pub fn init(&mut self) -> Result<(), TimeError> {
        self.battery_ok = CmosRtc::is_battery_healthy();

        match CmosRtc::read_datetime() {
            Ok(dt) => {
                klog_info!(
                    "[time] CMOS RTC initialized: {} (Battery: {})",
                    dt.format_human(),
                    if self.battery_ok { "OK" } else { "LOW/DEPLETED" }
                );
                self.initial_rtc_dt = Some(dt);
                WallClock::sync_with_rtc();
            }
            Err(e) => {
                klog_info!("[time] CMOS RTC read notice: {} (Using baseline epoch)", e.as_str());
                WallClock::sync_with_rtc();
            }
        }

        let cur_dt = WallClock::now();
        klog_ok!(
            "[time] System Wall-Clock active: {} (UTC)",
            cur_dt.format_iso8601()
        );

        Ok(())
    }

    /// Returns the current civil date and time.
    #[inline]
    pub fn now(&self) -> DateTime {
        WallClock::now()
    }

    /// Returns the current Unix epoch timestamp in seconds.
    #[inline]
    pub fn now_timestamp(&self) -> u64 {
        WallClock::now_timestamp()
    }

    /// Returns monotonic uptime in milliseconds.
    #[inline]
    pub fn uptime_millis(&self) -> u64 {
        MonotonicClock::uptime_millis()
    }

    /// Returns monotonic uptime in seconds.
    #[inline]
    pub fn uptime_seconds(&self) -> u64 {
        MonotonicClock::uptime_seconds()
    }

    /// Checks if the CMOS battery is healthy.
    #[inline]
    pub fn is_battery_healthy(&self) -> bool {
        self.battery_ok
    }

    /// Creates a one-shot software timer.
    pub fn set_timeout(&mut self, duration_ms: u64) -> TimerId {
        let cur_ms = self.uptime_millis();
        self.timers.create_timer(TimerMode::OneShot, duration_ms, cur_ms)
    }

    /// Creates a recurring periodic software timer.
    pub fn set_interval(&mut self, interval_ms: u64) -> TimerId {
        let cur_ms = self.uptime_millis();
        self.timers.create_timer(TimerMode::Periodic, interval_ms, cur_ms)
    }

    /// Cancels a software timer.
    pub fn cancel_timer(&mut self, id: TimerId) -> bool {
        self.timers.cancel(id)
    }

    /// Polls and triggers any expired timers.
    pub fn poll_timers(&mut self) -> Vec<TimerId> {
        let cur_ms = self.uptime_millis();
        self.timers.poll_expired(cur_ms)
    }

    /// Checks if a specific timer has expired.
    pub fn is_timer_expired(&self, id: TimerId) -> bool {
        self.timers.is_expired(id)
    }

    pub fn timers_mut(&mut self) -> &mut TimerService {
        &mut self.timers
    }
}

pub static TIME_MANAGER: IrqSpinlock<Option<TimeManager>> = IrqSpinlock::new(None);

/// Returns the current wall-clock date and time.
pub fn now() -> DateTime {
    let lock = TIME_MANAGER.lock();
    lock.as_ref().map(|tm| tm.now()).unwrap_or(DateTime::UNIX_EPOCH)
}

/// Returns the current Unix timestamp in seconds.
pub fn timestamp() -> u64 {
    let lock = TIME_MANAGER.lock();
    lock.as_ref().map(|tm| tm.now_timestamp()).unwrap_or(0)
}

/// Returns the system uptime in milliseconds.
pub fn uptime_ms() -> u64 {
    let lock = TIME_MANAGER.lock();
    lock.as_ref().map(|tm| tm.uptime_millis()).unwrap_or(0)
}

/// Returns the current formatted ISO-8601 time string.
pub fn format_now_iso8601() -> String {
    now().format_iso8601()
}

/// Executes a closure with mutable access to the global `TimeManager`.
pub fn with_time_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut TimeManager) -> R,
{
    let mut lock = TIME_MANAGER.lock();
    lock.as_mut().map(f)
}
