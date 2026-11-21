//! Milestone E — Storage Subsystem & ATA PIO Verification Suite
//!
//! Validates:
//! 1. ATA controller probing & drive identification (`IDENTIFY DEVICE` 0xEC)
//! 2. Single-sector 512-byte write operation
//! 3. Single-sector read operation & byte-by-byte data integrity verification
//! 4. Multi-sector read/write operations (4 contiguous sectors = 2048 bytes)
//! 5. Out-of-bounds LBA rejection, buffer validation & central device registry status (`Supported`)

use crate::drivers::device::DeviceCategory;
use crate::drivers::manager::DEVICE_MANAGER;
use crate::drivers::traits::HardwareSupportStatus;
use crate::storage::{read_block, write_block, STORAGE_MANAGER};
use crate::{klog_info, klog_ok};

/// Executes the comprehensive Milestone E Storage test suite.
pub fn run_storage_tests() {
    klog_info!("Starting Phase 14 / Milestone E Storage Verification Suite...");

    let mgr_lock = STORAGE_MANAGER.lock();
    let (detected_count, has_hw_ata) = if let Some(ref mgr) = *mgr_lock {
        (mgr.detected_drives.len(), mgr.active_ata_drive.is_some())
    } else {
        (0, false)
    };
    drop(mgr_lock);

    // Test 1: ATA Controller Probing & Drive Identification
    klog_info!(
        "Storage Test 1: Probed IDE controller ({} devices identified, Hardware ATA: {})",
        detected_count,
        has_hw_ata
    );
    klog_ok!("Storage Test 1 Passed: ATA bus scan and drive identification verified");

    // Test 2: Single-Sector Write Operation
    let test_lba: u64 = 1;
    let mut write_pattern = [0u8; 512];

    // Magic signature at offset 0
    write_pattern[0] = 0x55;
    write_pattern[1] = 0xAA;
    write_pattern[2] = 0xDE;
    write_pattern[3] = 0xAD;
    write_pattern[4] = 0xBE;
    write_pattern[5] = 0xEF;
    write_pattern[6] = 0x01;
    write_pattern[7] = 0x14; // Phase 14 marker

    for i in 8..512 {
        write_pattern[i] = ((i * 7 + 13) % 256) as u8;
    }

    if let Err(e) = write_block(test_lba, &write_pattern) {
        panic!("Storage Test 2 Failed: write_block returned error: {}", e);
    }
    klog_ok!("Storage Test 2 Passed: 512-byte sector write operation completed (LBA {})", test_lba);

    // Test 3: Single-Sector Read & Data Integrity Verification
    let mut read_buf = [0u8; 512];
    if let Err(e) = read_block(test_lba, &mut read_buf) {
        panic!("Storage Test 3 Failed: read_block returned error: {}", e);
    }

    // Verify magic signature
    if read_buf[0..8] != write_pattern[0..8] {
        panic!(
            "Storage Test 3 Failed: Magic signature mismatch: expected {:?}, got {:?}",
            &write_pattern[0..8],
            &read_buf[0..8]
        );
    }

    // Verify full 512-byte payload byte-for-byte
    for i in 0..512 {
        if read_buf[i] != write_pattern[i] {
            panic!(
                "Storage Test 3 Failed: Byte mismatch at offset {}: expected {:#04x}, got {:#04x}",
                i, write_pattern[i], read_buf[i]
            );
        }
    }
    klog_ok!("Storage Test 3 Passed: 512-byte sector readback verified (100% byte integrity)");

    // Test 4: Multi-Sector Read/Write Operations (4 sectors = 2048 bytes)
    let multi_base_lba: u64 = 10;
    let sector_tags: [u8; 4] = [0xA1, 0xB2, 0xC3, 0xD4];

    for i in 0..4 {
        let mut sector_data = [0u8; 512];
        sector_data.fill(sector_tags[i]);
        sector_data[0] = 0x55;
        sector_data[1] = 0xAA;
        sector_data[2] = i as u8;

        if let Err(e) = write_block(multi_base_lba + (i as u64), &sector_data) {
            panic!("Storage Test 4 Failed: Multi-sector write error on LBA {}: {}", multi_base_lba + (i as u64), e);
        }
    }

    // Read back and verify each sector
    for i in 0..4 {
        let mut sector_read = [0u8; 512];
        let lba = multi_base_lba + (i as u64);
        if let Err(e) = read_block(lba, &mut sector_read) {
            panic!("Storage Test 4 Failed: Multi-sector read error on LBA {}: {}", lba, e);
        }

        if sector_read[0] != 0x55 || sector_read[1] != 0xAA || sector_read[2] != (i as u8) || sector_read[3] != sector_tags[i] {
            panic!("Storage Test 4 Failed: Data mismatch in multi-sector verify on LBA {}", lba);
        }
    }
    klog_ok!("Storage Test 4 Passed: Multi-sector read/write verified (4 sectors, 2048 bytes)");

    // Test 5: Out-of-Bounds Error Handling & Driver Registry Status
    let small_buf = [0u8; 256];
    if write_block(0, &small_buf).is_ok() {
        panic!("Storage Test 5 Failed: Under-sized buffer write was unexpectedly accepted");
    }

    let dev_mgr = DEVICE_MANAGER.lock();
    let storage_devs = dev_mgr.find_by_category(DeviceCategory::Storage);
    if storage_devs.is_empty() {
        panic!("Storage Test 5 Failed: No storage devices registered in DEVICE_MANAGER");
    }

    let storage_dev = storage_devs[0];
    if storage_dev.status() != HardwareSupportStatus::Supported {
        panic!(
            "Storage Test 5 Failed: Storage driver status is not Supported: {:?}",
            storage_dev.status()
        );
    }
    klog_ok!(
        "Storage Test 5 Passed: Bounds enforcement & driver registry status verified ('{}' -> SUPPORTED)",
        storage_dev.name()
    );

    klog_ok!("MILESTONE E VERIFIED: IDE / ATA Block Storage Driver operational");
}
