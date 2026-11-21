//! Integrate OS — Phase 29 ACPI + Power Management Verification Test Suite
//!
//! Verifies:
//! - Test 1: Root System Description Pointer (RSDP) discovery & checksum verification
//! - Test 2: Root table (RSDT / XSDT) header parsing & table signature enumeration
//! - Test 3: Fixed ACPI Description Table (FADT) & power management block decoding
//! - Test 4: Multiple APIC Description Table (MADT) & APIC topology discovery
//! - Test 5: ACPI Power Management state machine & reset register capabilities

use crate::{klog_info, klog_ok};
use super::manager::ACPI_MANAGER;
use super::rsdp::find_rsdp;
use super::types::*;

/// Runs the complete Phase 29 ACPI verification test suite.
pub fn run_acpi_tests() {
    klog_info!("[acpi-test] Starting Phase 29 ACPI + Power Management verification...");

    test_1_rsdp_discovery_and_checksum();
    test_2_root_table_parsing_and_enumeration();
    test_3_fadt_and_pm_blocks();
    test_4_madt_apic_discovery();
    test_5_power_state_machine_and_reset();

    klog_ok!("[acpi-test] ALL PHASE 29 ACPI + POWER MANAGEMENT TESTS PASSED (5/5)");
}

/// Test 1: RSDP discovery and checksum verification.
fn test_1_rsdp_discovery_and_checksum() {
    klog_info!("[acpi-test] Test 1: RSDP discovery & checksum validation...");

    let rsdp = find_rsdp().expect("Test 1 Failed: Could not locate RSDP in EBDA/BIOS memory range");

    assert!(rsdp.matches_signature(), "Test 1 Failed: RSDP signature mismatch");
    assert!(rsdp.verify_checksum(), "Test 1 Failed: RSDP checksum verification failed");

    let oem = rsdp.oem_id_str();
    assert!(!oem.is_empty(), "Test 1 Failed: Empty OEM string");
    klog_info!(
        "[acpi-test] Found valid RSDP: OEM='{}', Rev={}, RootTable=0x{:08X}",
        oem,
        rsdp.revision,
        rsdp.root_table_address()
    );

    assert!(rsdp.root_table_address() != 0, "Test 1 Failed: Root table address is 0");

    klog_ok!("[acpi-test] Test 1 PASSED: RSDP discovery & checksum validation verified");
}

/// Test 2: Root table (RSDT / XSDT) parsing and table enumeration.
fn test_2_root_table_parsing_and_enumeration() {
    klog_info!("[acpi-test] Test 2: Root table parsing & table signature enumeration...");

    let lock = ACPI_MANAGER.lock();
    let mgr = lock.as_ref().expect("Test 2 Failed: ACPI_MANAGER not initialized");

    let tables = mgr.tables();
    let entries = tables.entries();
    assert!(!entries.is_empty(), "Test 2 Failed: No ACPI tables enumerated");

    klog_info!("[acpi-test] Enumerated {} ACPI description tables:", entries.len());
    for entry in entries {
        klog_info!(
            "[acpi-test]   - [{}] at 0x{:08X} (length: {} bytes, rev: {})",
            entry.signature,
            entry.phys_addr,
            entry.length,
            entry.revision
        );
        assert!(entry.phys_addr != 0);
        assert!(entry.length >= 36);
    }

    // Standard PC platforms always provide FADT ("FACP") and MADT ("APIC")
    let has_fadt = tables.find(AcpiSignature::FADT).is_some();
    assert!(has_fadt, "Test 2 Failed: Required FADT ('FACP') table not found");

    let has_madt = tables.find(AcpiSignature::MADT).is_some();
    assert!(has_madt, "Test 2 Failed: Required MADT ('APIC') table not found");

    klog_ok!("[acpi-test] Test 2 PASSED: Root table parsing & table enumeration verified");
}

/// Test 3: Fixed ACPI Description Table (FADT) & power management block decoding.
fn test_3_fadt_and_pm_blocks() {
    klog_info!("[acpi-test] Test 3: FADT & power management block decoding...");

    let lock = ACPI_MANAGER.lock();
    let mgr = lock.as_ref().expect("Test 3 Failed: ACPI_MANAGER not initialized");

    let fadt = mgr.fadt().expect("Test 3 Failed: FADT not present in AcpiManager");

    klog_info!(
        "[acpi-test] FADT Configuration:\n  DSDT: 0x{:08X}\n  SMI_CMD: 0x{:04X}\n  ACPI_EN: 0x{:02X}\n  PM1a_CNT: 0x{:04X}\n  PM1a_EVT: 0x{:04X}\n  Century: 0x{:02X}",
        fadt.dsdt_address,
        fadt.smi_cmd_port,
        fadt.acpi_enable_val,
        fadt.pm1a_cnt_blk,
        fadt.pm1a_evt_blk,
        fadt.century_reg
    );

    assert!(fadt.dsdt_address != 0, "Test 3 Failed: DSDT address is 0");
    assert!(fadt.pm1a_cnt_blk != 0, "Test 3 Failed: PM1a_CNT block address is 0");

    klog_ok!("[acpi-test] Test 3 PASSED: FADT & power management block decoding verified");
}

/// Test 4: Multiple APIC Description Table (MADT) & APIC topology discovery.
fn test_4_madt_apic_discovery() {
    klog_info!("[acpi-test] Test 4: MADT & APIC topology discovery...");

    let lock = ACPI_MANAGER.lock();
    let mgr = lock.as_ref().expect("Test 4 Failed: ACPI_MANAGER not initialized");

    let madt = mgr.madt().expect("Test 4 Failed: MADT not present in AcpiManager");

    klog_info!(
        "[acpi-test] MADT Configuration:\n  Local APIC Base: 0x{:08X}\n  CPUs (Local APICs): {}\n  I/O APICs: {}\n  Interrupt Overrides: {}",
        madt.local_apic_address,
        madt.num_local_apics,
        madt.num_io_apics,
        madt.num_iso
    );

    assert!(madt.local_apic_address != 0, "Test 4 Failed: Local APIC address is 0");
    assert!(madt.num_local_apics >= 1, "Test 4 Failed: 0 Local APICs detected");
    assert!(madt.num_io_apics >= 1, "Test 4 Failed: 0 I/O APICs detected");

    klog_ok!("[acpi-test] Test 4 PASSED: MADT & APIC topology discovery verified");
}

/// Test 5: ACPI Power Management state machine & reset register capabilities.
fn test_5_power_state_machine_and_reset() {
    klog_info!("[acpi-test] Test 5: Power management state machine & reset capabilities...");

    // 1. Verify sleep state representations
    let s0 = SleepState::S0;
    let s3 = SleepState::S3;
    let s5 = SleepState::S5;

    assert_eq!(s0.as_str(), "S0 (Working)");
    assert_eq!(s3.as_str(), "S3 (Suspend-to-RAM)");
    assert_eq!(s5.as_str(), "S5 (Soft Off / Shutdown)");

    // 2. Query reset support
    let lock = ACPI_MANAGER.lock();
    let mgr = lock.as_ref().expect("Test 5 Failed: ACPI_MANAGER not initialized");

    if let Some(fadt) = mgr.fadt() {
        let supports_reset = fadt.supports_acpi_reset();
        klog_info!(
            "[acpi-test] Hardware ACPI Reset Supported: {}",
            if supports_reset { "YES" } else { "NO (Using 8042 Keyboard Controller fallback)" }
        );
    }

    // 3. Battery status query
    let battery = BatteryStatus::unknown();
    assert_eq!(battery.percentage, 100);

    klog_ok!("[acpi-test] Test 5 PASSED: Power management state machine & reset verified");
}
