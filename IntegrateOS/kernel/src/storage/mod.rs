//! Integrate OS — Storage Subsystem Architecture (Milestone E)
//!
//! Provides:
//! - IDE / ATA PIO controller hardware driver (`ata.rs`)
//! - In-memory RAM disk block device (`ramdisk.rs`)
//! - Central storage manager (`STORAGE_MANAGER`)
//! - Milestone E verification test suite (`tests.rs`)

pub mod ata;
pub mod ramdisk;
pub mod ssd;
pub mod tests;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::drivers::manager::DEVICE_MANAGER;
use crate::storage::ata::{
    identify_drive, read_sector_pio, write_sector_pio, AtaBus, AtaDeviceInfo, AtaDriveId,
    AtaDriveType,
};
use crate::storage::ramdisk::{RamDisk, DEFAULT_RAMDISK_SECTORS};
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

static STORAGE_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Storage Subsystem Manager.
pub struct StorageManager {
    pub detected_drives: Vec<AtaDeviceInfo>,
    pub active_ata_drive: Option<AtaDeviceInfo>,
    pub ramdisk: RamDisk,
}

pub static STORAGE_MANAGER: IrqSpinlock<Option<StorageManager>> = IrqSpinlock::new(None);

/// Initializes the storage subsystem, probes all 4 IDE channels, and configures active storage.
pub fn init() {
    let mut detected = Vec::new();
    let mut active_ata = None;

    // 1. Probe all standard IDE channels
    let probe_targets = [
        (AtaBus::Primary, AtaDriveId::Master, "Primary Master"),
        (AtaBus::Primary, AtaDriveId::Slave, "Primary Slave"),
        (AtaBus::Secondary, AtaDriveId::Master, "Secondary Master"),
        (AtaBus::Secondary, AtaDriveId::Slave, "Secondary Slave"),
    ];

    for (bus, drive, label) in probe_targets {
        if let Some(info) = identify_drive(bus, drive) {
            match info.drive_type {
                AtaDriveType::Ata => {
                    klog_ok!(
                        "ATA Disk Detected on {}: '{}' (Serial: '{}', {} sectors / {} MiB)",
                        label,
                        info.model,
                        info.serial,
                        info.total_sectors,
                        (info.total_sectors * 512) / (1024 * 1024)
                    );
                    if active_ata.is_none() {
                        active_ata = Some(info.clone());
                    }
                }
                AtaDriveType::Atapi => {
                    klog_info!("ATAPI Device Detected on {}: '{}'", label, info.model);
                }
                AtaDriveType::None => {}
            }
            detected.push(info);
        }
    }

    let mgr = StorageManager {
        detected_drives: detected,
        active_ata_drive: active_ata,
        ramdisk: RamDisk::new(DEFAULT_RAMDISK_SECTORS),
    };

    *STORAGE_MANAGER.lock() = Some(mgr);
    STORAGE_INITIALIZED.store(true, Ordering::Release);

    // 2. Update Device Registry status for Storage Category
    let dev_mgr = DEVICE_MANAGER.lock();
    let storage_devs = dev_mgr.find_by_category(crate::drivers::device::DeviceCategory::Storage);
    klog_ok!(
        "Storage Subsystem Initialized: Central registry tracking {} storage device(s)",
        storage_devs.len()
    );
}

/// Returns `true` if the storage subsystem has completed initialization.
#[inline(always)]
pub fn is_storage_initialized() -> bool {
    STORAGE_INITIALIZED.load(Ordering::Acquire)
}

/// Returns the total sector count of the primary block device.
pub fn total_blocks() -> u64 {
    let mgr_lock = STORAGE_MANAGER.lock();
    if let Some(ref mgr) = *mgr_lock {
        if let Some(ref ata) = mgr.active_ata_drive {
            ata.total_sectors
        } else {
            mgr.ramdisk.total_sectors()
        }
    } else {
        DEFAULT_RAMDISK_SECTORS as u64
    }
}

/// Reads a 512-byte block from the active storage controller into `buf`.
pub fn read_block(block: u64, buf: &mut [u8]) -> Result<(), &'static str> {
    if buf.len() < 512 {
        return Err("Destination buffer smaller than 512 bytes");
    }

    let mut mgr_lock = STORAGE_MANAGER.lock();
    let mgr = mgr_lock.as_mut().ok_or("Storage manager not initialized")?;

    if let Some(ata) = mgr.active_ata_drive.clone() {
        drop(mgr_lock);
        let mut temp_buf = [0u8; 512];
        read_sector_pio(ata.bus, ata.drive, block as u32, &mut temp_buf)?;
        buf[..512].copy_from_slice(&temp_buf);
        Ok(())
    } else {
        let mut temp_buf = [0u8; 512];
        mgr.ramdisk.read_sector(block, &mut temp_buf)?;
        buf[..512].copy_from_slice(&temp_buf);
        Ok(())
    }
}

/// Writes a 512-byte block to the active storage controller from `buf`.
pub fn write_block(block: u64, buf: &[u8]) -> Result<(), &'static str> {
    if buf.len() < 512 {
        return Err("Source buffer smaller than 512 bytes");
    }

    let mut mgr_lock = STORAGE_MANAGER.lock();
    let mgr = mgr_lock.as_mut().ok_or("Storage manager not initialized")?;

    if let Some(ata) = mgr.active_ata_drive.clone() {
        drop(mgr_lock);
        let mut temp_buf = [0u8; 512];
        temp_buf.copy_from_slice(&buf[..512]);
        write_sector_pio(ata.bus, ata.drive, block as u32, &temp_buf)
    } else {
        let mut temp_buf = [0u8; 512];
        temp_buf.copy_from_slice(&buf[..512]);
        mgr.ramdisk.write_sector(block, &temp_buf)
    }
}
