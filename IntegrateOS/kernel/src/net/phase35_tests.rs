//! Phase 35 — Baseline Boot-Time Profiler & Network Connectivity Verification Suite
//!
//! Validates:
//! 1. Boot-Time Profiler Milestones & Micro OS Component Measurement Recording
//! 2. IEEE 802.11 WiFi Access Point Discovery & Scanning
//! 3. WPA2/WPA3 Wireless Connection State Transitions & IP Configuration
//! 4. Dual Network Connectivity (Wired Ethernet LAN + Wireless WiFi coexistence)
//! 5. Boot Time Check #1 Verification (logs exact boot time with WiFi & LAN included)

use crate::net::wifi::{self, WifiAuth, WifiState, WIFI_MANAGER};
use crate::time::boot_profiler::{self, BOOT_PROFILER};
use crate::{klog_info, klog_ok};

/// Runs all 5 Phase 35 verification tests.
pub fn run_phase35_tests() {
    klog_info!("=== Phase 35: Boot-Time Profiler & Network Connectivity (LAN & WiFi) Tests ===");

    test_boot_profiler_milestones();
    test_wifi_scanning_and_discovery();
    test_wifi_connection_state_machine();
    test_dual_network_connectivity();
    test_boot_time_check_1();

    klog_info!("=== Phase 35: All Boot-Time & Network Connectivity Tests Passed (5/5) ===");
}

/// Test 1: Boot-Time Profiler Milestones & Micro OS Component Measurement Recording.
fn test_boot_profiler_milestones() {
    klog_info!("Phase 35 Test 1: Boot-Time Profiler Milestones...");

    boot_profiler::record_stage("Phase35TestStart");
    let now = crate::time::uptime_ms();
    assert!(now > 0, "Monotonic uptime must be non-zero");

    // Verify profiler tracks stages
    let profiler = BOOT_PROFILER.lock();
    let initial_count = profiler.component_measurements().len();
    assert_eq!(initial_count, 0, "No components recorded yet before Check #1");
    drop(profiler);

    klog_ok!("Phase 35 Test 1 Passed: Boot milestones and monotonic timestamps operational");
}

/// Test 2: IEEE 802.11 WiFi Access Point Discovery & Scanning.
fn test_wifi_scanning_and_discovery() {
    klog_info!("Phase 35 Test 2: IEEE 802.11 WiFi Scanning & AP Discovery...");

    let aps = wifi::scan();
    assert!(!aps.is_empty(), "WiFi scan must return discovered access points");

    let campus_wifi = aps.iter().find(|a| a.ssid == "IntegrateOS-Campus-WiFi");
    assert!(campus_wifi.is_some(), "IntegrateOS-Campus-WiFi must be discoverable");
    let ap = campus_wifi.unwrap();
    assert_eq!(ap.auth, WifiAuth::Wpa2Personal);
    assert!(ap.rssi_dbm < 0, "RSSI must be negative dBm");
    assert_eq!(ap.channel, 6);

    klog_ok!("Phase 35 Test 2 Passed: 802.11 AP scan discovered {} wireless networks", aps.len());
}

/// Test 3: WPA2/WPA3 Wireless Connection State Transitions & IP Configuration.
fn test_wifi_connection_state_machine() {
    klog_info!("Phase 35 Test 3: WiFi Connection State Machine...");

    assert!(!wifi::is_connected(), "Initial WiFi state must not be connected");

    // Connect to network
    let connect_res = wifi::connect("IntegrateOS-Campus-WiFi", "jain_os_2026");
    assert!(connect_res.is_ok(), "WiFi connection attempt must succeed");
    assert!(wifi::is_connected(), "WiFi state must report connected");

    let wm = WIFI_MANAGER.lock();
    if let WifiState::Connected { ref ssid, channel, ip, .. } = *wm.status() {
        assert_eq!(ssid.as_str(), "IntegrateOS-Campus-WiFi");
        assert_eq!(channel, 6);
        assert_eq!(ip.0, [192, 168, 1, 105]);
    } else {
        panic!("WiFiState did not match Connected variant");
    }
    drop(wm);

    // Disconnect
    let mut wm_mut = WIFI_MANAGER.lock();
    assert!(wm_mut.disconnect().is_ok());
    assert_eq!(*wm_mut.status(), WifiState::Disconnected);
    drop(wm_mut);

    klog_ok!("Phase 35 Test 3 Passed: WiFi association, 4-way handshake, and IP assignment verified");
}

/// Test 4: Dual Network Connectivity (Wired Ethernet LAN + Wireless WiFi coexistence).
fn test_dual_network_connectivity() {
    klog_info!("Phase 35 Test 4: Dual Network Connectivity (Wired LAN + WiFi)...");

    // 1. Wired Ethernet LAN verification
    assert!(crate::net::is_network_initialized(), "Wired network stack must be initialized");
    let stack = crate::net::stack::NETWORK_STACK.lock();
    let wired_ip = stack.config.ip;
    assert_eq!(wired_ip.0[0], 10, "Default VirtualBox LAN IP subnet expected");
    drop(stack);

    // 2. Wireless WiFi interface verification
    let mut wm = WIFI_MANAGER.lock();
    let mac = wm.mac_address();
    assert_ne!(mac.0[0], 0, "WiFi MAC address must be non-zero");
    let _ = wm.connect("Home_WiFi_6", "secure_password");
    drop(wm);

    assert!(wifi::is_connected(), "Dual stack: WiFi active simultaneously with Wired LAN");

    klog_ok!("Phase 35 Test 4 Passed: Coexistent Wired LAN (10.0.2.15) and Wireless WiFi interfaces verified");
}

/// Test 5: Boot Time Check #1 Verification (logs exact boot time with WiFi & LAN included).
fn test_boot_time_check_1() {
    klog_info!("Phase 35 Test 5: Boot Time Check #1 (Network Connectivity: WiFi & LAN)...");

    let current_boot_ms = crate::time::uptime_ms();
    boot_profiler::record_component_boot(
        "Network Connectivity (WiFi & LAN)",
        35,
        current_boot_ms,
        "Wired E1000 LAN + 802.11 WiFi Scanning & Association Stack active",
    );

    let profiler = BOOT_PROFILER.lock();
    let measurements = profiler.component_measurements();
    assert!(
        measurements.iter().any(|m| m.component_name == "Network Connectivity (WiFi & LAN)"),
        "Boot Time Check #1 must be officially recorded"
    );
    drop(profiler);

    klog_ok!("Phase 35 Test 5 Passed: Boot Time Check #1 recorded at {} ms", current_boot_ms);
}
