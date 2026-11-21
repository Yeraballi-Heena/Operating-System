//! Integrate OS — PCI / PCIe Subsystem Verification Test Suite
//!
//! Validates:
//! 1. Configuration space mechanism #1 address encoding, dword/word/byte alignment & host bridge read
//! 2. Standard PCI configuration header parsing & command register bus mastering manipulation
//! 3. Base Address Register (BAR0..BAR5) sizing, Memory-Mapped I/O (MMIO) & Port I/O decoding
//! 4. Capability linked-list traversal (Power Management, MSI, PCIe) and boundary safety
//! 5. Full physical bus enumeration, device inventory verification & class-based hardware queries

use crate::pci::address::PciAddress;
use crate::pci::bar::Bar;
use crate::pci::capabilities::parse_capabilities;
use crate::pci::manager::PCI_MANAGER;
use crate::pci::types::*;
use crate::{klog_info, klog_ok};

/// Executes the full Phase 24 PCI / PCIe verification test suite.
pub fn run_pci_tests() {
    klog_info!("[pci-test] Starting Phase 24 PCI / PCIe verification...");

    test_config_space_access();
    test_device_header_and_commands();
    test_bar_probing_and_decoding();
    test_capability_linked_list();
    test_bus_enumeration_and_discovery();

    klog_ok!("[pci-test] ALL PHASE 24 PCI / PCIE TESTS PASSED (5/5)");
}

/// Test 1: Configuration space mechanism #1 addressing, dword/word/byte alignment & host bridge ID.
fn test_config_space_access() {
    klog_info!("[pci-test] Test 1: Configuration mechanism #1 & host bridge access...");

    let host_bridge = PciAddress::new(0, 0, 0);
    assert_eq!(host_bridge.bus, 0);
    assert_eq!(host_bridge.device, 0);
    assert_eq!(host_bridge.function, 0);
    assert_eq!(host_bridge.to_string(), "00:00.0");

    // Read 32-bit dword at offset 0 (Vendor ID [15:0] and Device ID [31:16])
    let dword0 = host_bridge.read_u32(0);
    let vendor_id = (dword0 & 0xFFFF) as u16;
    let device_id = ((dword0 >> 16) & 0xFFFF) as u16;

    // Standard x86 host bridges (Intel 82440FX/BX: 0x8086)
    assert_eq!(vendor_id, 0x8086, "Host bridge vendor must be Intel (0x8086)");
    assert_ne!(device_id, 0xFFFF, "Host bridge device ID must be valid");

    // Verify 16-bit word read matches dword
    let word_vendor = host_bridge.read_u16(PCI_VENDOR_ID);
    let word_device = host_bridge.read_u16(PCI_DEVICE_ID);
    assert_eq!(word_vendor, vendor_id);
    assert_eq!(word_device, device_id);

    // Verify 8-bit byte read matches 16-bit word
    let byte_low = host_bridge.read_u8(PCI_VENDOR_ID);
    let byte_high = host_bridge.read_u8(PCI_VENDOR_ID + 1);
    assert_eq!(byte_low, (vendor_id & 0xFF) as u8);
    assert_eq!(byte_high, (vendor_id >> 8) as u8);

    klog_ok!("[pci-test] Test 1 PASSED: Config space mechanism #1 & byte alignment verified");
}

/// Test 2: Standard PCI configuration header parsing and command register bus mastering.
fn test_device_header_and_commands() {
    klog_info!("[pci-test] Test 2: Configuration header parsing & command register...");

    let host_bridge = PciAddress::new(0, 0, 0);

    let class_code = host_bridge.read_u8(PCI_CLASS);
    let subclass = host_bridge.read_u8(PCI_SUBCLASS);

    // Host Bridge is Class 0x06 (Bridge), Subclass 0x00 (Host Bridge)
    assert_eq!(class_code, 0x06, "00:00.0 must be a Bridge Device (Class 0x06)");
    assert_eq!(subclass, 0x00, "00:00.0 must be a Host Bridge (Subclass 0x00)");

    // Command Register manipulation
    let original_cmd = host_bridge.read_u16(PCI_COMMAND);

    // Set Bus Master bit
    host_bridge.write_u16(PCI_COMMAND, original_cmd | PCI_COMMAND_BUS_MASTER);
    let modified_cmd = host_bridge.read_u16(PCI_COMMAND);
    assert_eq!(
        modified_cmd & PCI_COMMAND_BUS_MASTER,
        PCI_COMMAND_BUS_MASTER,
        "Bus Master bit must be asserted"
    );

    // Restore original command register
    host_bridge.write_u16(PCI_COMMAND, original_cmd);
    assert_eq!(host_bridge.read_u16(PCI_COMMAND), original_cmd);

    klog_ok!("[pci-test] Test 2 PASSED: Header parsing and command register verified");
}

/// Test 3: Base Address Register (BAR0..BAR5) probing, sizing, and MMIO decoding.
fn test_bar_probing_and_decoding() {
    klog_info!("[pci-test] Test 3: Base Address Registers (BAR) probing & MMIO sizing...");

    let mgr = PCI_MANAGER.lock();
    let mut found_valid_bar = false;

    for dev in &mgr.devices {
        for bar_opt in &dev.bars {
            if let Some(bar) = bar_opt {
                found_valid_bar = true;
                match *bar {
                    Bar::Memory { address, size, is_64bit, .. } => {
                        assert!(size > 0, "MMIO BAR size must be non-zero");
                        assert!(size.is_power_of_two(), "BAR size must be power of two: {}", size);
                        if is_64bit {
                            assert!(address < 0xFFFF_FFFF_FFFF_FFFF);
                        } else {
                            assert!(address < 0x1_0000_0000);
                        }
                    }
                    Bar::Io { port, size } => {
                        assert!(size > 0, "I/O BAR size must be non-zero");
                        assert!(size.is_power_of_two(), "I/O BAR size must be power of two");
                        assert!(port > 0, "I/O BAR port must be non-zero");
                    }
                }
            }
        }
    }

    assert!(found_valid_bar, "At least one populated BAR must exist across PCI devices");
    klog_ok!("[pci-test] Test 3 PASSED: BAR probing, sizing and MMIO decoding verified");
}

/// Test 4: Capability linked-list traversal and structure parsing.
fn test_capability_linked_list() {
    klog_info!("[pci-test] Test 4: Capability linked list traversal...");

    let mgr = PCI_MANAGER.lock();
    let mut checked_devices = 0;

    for dev in &mgr.devices {
        let caps = parse_capabilities(&dev.addr);
        checked_devices += 1;

        // Verify all parsed capabilities have valid configuration space offsets (>= 0x40)
        for cap in caps {
            assert!(
                cap.offset >= 0x40,
                "Capability offset ({:#x}) must be in extended config space",
                cap.offset
            );
        }
    }

    assert!(checked_devices > 0, "At least one device checked for capabilities");
    klog_ok!("[pci-test] Test 4 PASSED: Capability linked list traversal verified");
}

/// Test 5: Full PCI bus enumeration, hardware inventory, and driver framework integration.
fn test_bus_enumeration_and_discovery() {
    klog_info!("[pci-test] Test 5: PCI bus enumeration & device registry discovery...");

    let mgr = PCI_MANAGER.lock();
    assert!(mgr.count() >= 3, "Expected at least 3 PCI devices (Host Bridge, IDE, Display)");

    // Query 1: Host Bridge (0x06:0x00)
    let bridges = mgr.find_by_class(0x06, 0x00);
    assert!(!bridges.is_empty(), "Must find at least one Host Bridge");
    assert_eq!(bridges[0].vendor_id, 0x8086);

    // Query 2: Display Controller (Class 0x03)
    let displays = mgr.find_by_class(0x03, 0x00);
    assert!(!displays.is_empty(), "Must find at least one VGA-Compatible Display Controller");

    // Query 3: Mass Storage Controller (Class 0x01)
    let storage = mgr.find_by_class(0x01, 0x01);
    assert!(!storage.is_empty(), "Must find IDE Mass Storage Controller");

    // Verify Driver Framework status is now Supported
    let dev_mgr = crate::drivers::manager::DEVICE_MANAGER.lock();
    let pci_drivers = dev_mgr.find_by_category(crate::drivers::device::DeviceCategory::Pci);
    assert!(!pci_drivers.is_empty(), "PciFoundationDevice must be registered in Device Manager");
    assert_eq!(
        pci_drivers[0].status(),
        crate::drivers::traits::HardwareSupportStatus::Supported,
        "PCI driver status must be Supported"
    );

    klog_ok!("[pci-test] Test 5 PASSED: Full bus enumeration & device discovery verified");
}
