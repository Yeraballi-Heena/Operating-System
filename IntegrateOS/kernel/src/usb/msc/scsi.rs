//! Integrate OS — SCSI Command Set for USB Mass Storage Devices
//!
//! Provides SCSI Primary Commands (SPC) and Block Commands (SBC) CDB builders
//! and parser structures.

use alloc::string::String;
use super::super::types::*;

pub const SCSI_TEST_UNIT_READY: u8 = 0x00;
pub const SCSI_REQUEST_SENSE: u8 = 0x03;
pub const SCSI_INQUIRY: u8 = 0x12;
pub const SCSI_READ_CAPACITY_10: u8 = 0x25;
pub const SCSI_READ_10: u8 = 0x28;
pub const SCSI_WRITE_10: u8 = 0x2A;

/// Builds a 16-byte CDB for the standard SCSI `INQUIRY` command.
pub fn build_inquiry_cdb(alloc_len: u8) -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = SCSI_INQUIRY;
    cdb[4] = alloc_len; // 36 bytes standard
    cdb
}

/// Builds a 16-byte CDB for the SCSI `TEST UNIT READY` command.
pub fn build_test_unit_ready_cdb() -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = SCSI_TEST_UNIT_READY;
    cdb
}

/// Builds a 16-byte CDB for the SCSI `READ CAPACITY (10)` command.
pub fn build_read_capacity_cdb() -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = SCSI_READ_CAPACITY_10;
    cdb
}

/// Builds a 16-byte CDB for the SCSI `READ (10)` command.
pub fn build_read10_cdb(lba: u32, sector_count: u16) -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = SCSI_READ_10;
    cdb[2..6].copy_from_slice(&lba.to_be_bytes());
    cdb[7..9].copy_from_slice(&sector_count.to_be_bytes());
    cdb
}

/// Builds a 16-byte CDB for the SCSI `WRITE (10)` command.
pub fn build_write10_cdb(lba: u32, sector_count: u16) -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = SCSI_WRITE_10;
    cdb[2..6].copy_from_slice(&lba.to_be_bytes());
    cdb[7..9].copy_from_slice(&sector_count.to_be_bytes());
    cdb
}

/// Parsed SCSI standard `INQUIRY` data (36 bytes).
#[derive(Debug, Clone)]
pub struct ScsiInquiryData {
    pub device_type: u8,
    pub is_removable: bool,
    pub vendor_id: String,
    pub product_id: String,
    pub revision: String,
}

impl ScsiInquiryData {
    pub fn parse(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 36 {
            return Err(UsbError::BufferTooSmall);
        }

        let device_type = data[0] & 0x1F;
        let is_removable = (data[1] & 0x80) != 0;

        let vendor_bytes = &data[8..16];
        let product_bytes = &data[16..32];
        let rev_bytes = &data[32..36];

        let vendor_id = String::from_utf8_lossy(vendor_bytes).trim().into();
        let product_id = String::from_utf8_lossy(product_bytes).trim().into();
        let revision = String::from_utf8_lossy(rev_bytes).trim().into();

        Ok(Self {
            device_type,
            is_removable,
            vendor_id,
            product_id,
            revision,
        })
    }
}

/// Parsed SCSI `READ CAPACITY (10)` response (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScsiCapacityData {
    pub last_lba: u32,
    pub block_size: u32,
}

impl ScsiCapacityData {
    pub fn parse(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 8 {
            return Err(UsbError::BufferTooSmall);
        }

        let last_lba = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let block_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        Ok(Self {
            last_lba,
            block_size,
        })
    }

    /// Total storage capacity in bytes.
    pub fn total_capacity_bytes(&self) -> u64 {
        ((self.last_lba as u64) + 1) * (self.block_size as u64)
    }
}
