//! Integrate OS — Phase 28 RTC + Time Verification Test Suite
//!
//! Verifies:
//! - Test 1: CMOS RTC hardware detection, status registers & BCD decoding
//! - Test 2: Gregorian calendar arithmetic, leap years, weekdays & ISO-8601 formatting
//! - Test 3: Unix Epoch timestamp bidirectional conversion roundtrip
//! - Test 4: Monotonic uptime clock tracking & non-decreasing guarantee
//! - Test 5: Software timer engine (One-shot, periodic & cancellation)

use crate::{klog_info, klog_ok};
use super::clock::{MonotonicClock, WallClock};
use super::cmos::CmosRtc;
use super::timer::{TimerMode, TimerService};
use super::types::*;

/// Runs the complete Phase 28 Time verification test suite.
pub fn run_time_tests() {
    klog_info!("[time-test] Starting Phase 28 RTC + Time verification...");

    test_1_cmos_rtc_detection_and_registers();
    test_2_calendar_arithmetic_and_iso8601();
    test_3_unix_timestamp_roundtrip();
    test_4_monotonic_uptime_clock();
    test_5_software_timer_engine();

    klog_ok!("[time-test] ALL PHASE 28 RTC + TIME TESTS PASSED (5/5)");
}

/// Test 1: CMOS RTC hardware detection, status registers and BCD decoding.
fn test_1_cmos_rtc_detection_and_registers() {
    klog_info!("[time-test] Test 1: CMOS RTC hardware detection & register reading...");

    // 1. Verify BCD <-> Binary conversions
    assert_eq!(CmosRtc::bcd_to_binary(0x59), 59);
    assert_eq!(CmosRtc::bcd_to_binary(0x00), 0);
    assert_eq!(CmosRtc::bcd_to_binary(0x26), 26);
    assert_eq!(CmosRtc::binary_to_bcd(59), 0x59);
    assert_eq!(CmosRtc::binary_to_bcd(26), 0x26);

    // 2. Read CMOS Status Register D (Battery power)
    let battery_ok = CmosRtc::is_battery_healthy();
    klog_info!(
        "[time-test] CMOS Battery Status: {}",
        if battery_ok { "HEALTHY / VALID" } else { "LOW / DEPLETED" }
    );

    // 3. Read current RTC date and time
    let dt = CmosRtc::read_datetime().expect("Test 1 Failed: CmosRtc::read_datetime returned error");
    klog_info!(
        "[time-test] CMOS RTC Readout: {} ({})",
        dt.format_human(),
        dt.weekday().as_str()
    );

    assert!(dt.is_valid(), "Test 1 Failed: RTC returned invalid DateTime: {:?}", dt);
    assert!(dt.year >= 2024, "Test 1 Failed: RTC year suspicious: {}", dt.year);
    assert!(dt.month >= 1 && dt.month <= 12);
    assert!(dt.day >= 1 && dt.day <= 31);
    assert!(dt.hour <= 23);
    assert!(dt.minute <= 59);
    assert!(dt.second <= 59);

    klog_ok!("[time-test] Test 1 PASSED: CMOS RTC hardware detection & register reading verified");
}

/// Test 2: Gregorian calendar arithmetic, leap years, weekdays & ISO-8601 formatting.
fn test_2_calendar_arithmetic_and_iso8601() {
    klog_info!("[time-test] Test 2: Gregorian calendar arithmetic, leap years & ISO-8601...");

    // 1. Leap year calculations
    assert!(DateTime::is_leap_year(2000)); // Divisible by 400 -> leap
    assert!(DateTime::is_leap_year(2024)); // Divisible by 4 -> leap
    assert!(!DateTime::is_leap_year(2100)); // Divisible by 100 but not 400 -> not leap
    assert!(!DateTime::is_leap_year(2025));
    assert!(!DateTime::is_leap_year(2026));

    // 2. Days in month
    assert_eq!(DateTime::days_in_month(2024, 2), 29); // Leap year Feb
    assert_eq!(DateTime::days_in_month(2026, 2), 28); // Normal year Feb
    assert_eq!(DateTime::days_in_month(2026, 1), 31);
    assert_eq!(DateTime::days_in_month(2026, 4), 30);
    assert_eq!(DateTime::days_in_month(2026, 11), 30);

    // 3. Weekday calculation (Sakamoto's algorithm)
    // 2026-11-15 is Sunday
    let dt_sunday = DateTime::new(2026, 11, 15, 12, 0, 0);
    assert_eq!(dt_sunday.weekday(), Weekday::Sunday);

    // 2024-01-01 was Monday
    let dt_mon = DateTime::new(2024, 1, 1, 0, 0, 0);
    assert_eq!(dt_mon.weekday(), Weekday::Monday);

    // 4. ISO-8601 formatting
    let dt_iso = DateTime::new(2026, 11, 15, 18, 30, 45);
    assert_eq!(dt_iso.format_iso8601(), "2026-11-15T18:30:45Z");
    assert_eq!(dt_iso.format_human(), "2026-11-15 18:30:45");

    klog_ok!("[time-test] Test 2 PASSED: Gregorian calendar arithmetic & ISO-8601 verified");
}

/// Test 3: Unix Epoch timestamp bidirectional conversion roundtrip.
fn test_3_unix_timestamp_roundtrip() {
    klog_info!("[time-test] Test 3: Unix Epoch timestamp bidirectional conversion...");

    // 1. Epoch baseline: 1970-01-01 00:00:00 = 0 seconds
    let epoch = DateTime::UNIX_EPOCH;
    assert_eq!(epoch.to_unix_timestamp(), 0);
    assert_eq!(DateTime::from_unix_timestamp(0), epoch);

    // 2. Known milestone: 2000-01-01 00:00:00 UTC = 946,684,800 seconds
    let y2k = DateTime::new(2000, 1, 1, 0, 0, 0);
    let y2k_ts = y2k.to_unix_timestamp();
    assert_eq!(y2k_ts, 946_684_800);
    assert_eq!(DateTime::from_unix_timestamp(946_684_800), y2k);

    // 3. Test roundtrip for multiple arbitrary dates
    let test_dates = [
        DateTime::new(1985, 10, 26, 9, 0, 0),
        DateTime::new(2020, 2, 29, 23, 59, 59), // Leap day
        DateTime::new(2026, 11, 15, 18, 12, 34),
        DateTime::new(2038, 1, 19, 3, 14, 7),   // 32-bit boundary limit
    ];

    for &orig in &test_dates {
        let ts = orig.to_unix_timestamp();
        let reconstructed = DateTime::from_unix_timestamp(ts);
        assert_eq!(
            orig, reconstructed,
            "Test 3 Failed: Roundtrip mismatch: {:?} -> {} -> {:?}",
            orig, ts, reconstructed
        );
    }

    klog_ok!("[time-test] Test 3 PASSED: Unix Epoch timestamp conversion roundtrip verified");
}

/// Test 4: Monotonic uptime clock tracking & non-decreasing guarantee.
fn test_4_monotonic_uptime_clock() {
    klog_info!("[time-test] Test 4: Monotonic uptime clock & non-decreasing guarantee...");

    let start_ticks = MonotonicClock::uptime_ticks();
    let start_ms = MonotonicClock::uptime_millis();

    assert!(start_ticks > 0, "Test 4 Failed: Uptime ticks is 0");
    assert_eq!(start_ms, start_ticks * 10);

    // Spin for ~30 milliseconds
    let target_ticks = start_ticks + 3;
    while MonotonicClock::uptime_ticks() < target_ticks {
        core::hint::spin_loop();
    }

    let end_ticks = MonotonicClock::uptime_ticks();
    let end_ms = MonotonicClock::uptime_millis();

    assert!(end_ticks >= target_ticks);
    assert!(end_ms > start_ms);

    // Verify Duration conversions
    let dur = MonotonicClock::uptime_duration();
    assert_eq!(dur.as_millis(), end_ms);
    assert_eq!(dur.as_micros(), end_ms * 1000);

    // Wall clock read
    let now_dt = WallClock::now();
    assert!(now_dt.is_valid());

    klog_ok!("[time-test] Test 4 PASSED: Monotonic uptime clock verified");
}

/// Test 5: Software timer engine (One-shot, periodic & cancellation).
fn test_5_software_timer_engine() {
    klog_info!("[time-test] Test 5: Software timer engine & deadline management...");

    let mut service = TimerService::new();
    let current_time: u64 = 10_000; // Simulated 10,000 ms baseline

    // 1. Create a 50 ms one-shot timer and a 30 ms periodic timer
    let t_oneshot = service.create_timer(TimerMode::OneShot, 50, current_time);
    let t_periodic = service.create_timer(TimerMode::Periodic, 30, current_time);

    assert_eq!(service.active_count(), 2);
    assert!(!service.is_expired(t_oneshot));
    assert!(!service.is_expired(t_periodic));

    // 2. Poll at current_time + 20 ms -> neither should expire
    let expired_20 = service.poll_expired(current_time + 20);
    assert!(expired_20.is_empty());
    assert!(!service.is_expired(t_oneshot));
    assert!(!service.is_expired(t_periodic));

    // 3. Poll at current_time + 35 ms -> periodic timer must expire
    let expired_35 = service.poll_expired(current_time + 35);
    assert_eq!(expired_35.len(), 1);
    assert_eq!(expired_35[0], t_periodic);
    assert!(service.is_expired(t_periodic));
    assert!(!service.is_expired(t_oneshot));

    // 4. Poll at current_time + 55 ms -> one-shot timer must expire
    let expired_55 = service.poll_expired(current_time + 55);
    assert_eq!(expired_55.len(), 1);
    assert_eq!(expired_55[0], t_oneshot);
    assert!(service.is_expired(t_oneshot));

    // 5. Test cancellation
    let t_cancel = service.create_timer(TimerMode::OneShot, 100, current_time);
    assert!(service.cancel(t_cancel));
    assert!(!service.cancel(9999)); // Unknown ID returns false

    klog_ok!("[time-test] Test 5 PASSED: Software timer engine verified");
}
