//! Integrate OS — USB Mass Storage Bulk-Only Transport (BOT) Protocol
//!
//! Implements the 31-byte Command Block Wrapper (CBW) and 13-byte Command Status
//! Wrapper (CSW) required for USB flash drives and external disks.

use super::super::types::*;

pub const CBW_SIGNATURE: u32 = 0x43425355; // "USBC" in little-endian
pub const CSW_SIGNATURE: u32 = 0x53425355; // "USBS" in little-endian

pub const CSW_STATUS_PASSED: u8 = 0x00;
pub const CSW_STATUS_FAILED: u8 = 0x01;
pub const CSW_STATUS_PHASE_ERROR: u8 = 0x02;

/// Standard 31-byte Command Block Wrapper (CBW).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBlockWrapper {
    pub d_cbw_signature: u32,
    pub d_cbw_tag: u32,
    pub d_cbw_data_transfer_length: u32,
    pub bm_cbw_flags: u8,
    pub b_cbw_lun: u8,
    pub b_cbw_cb_length: u8,
    pub cbw_cb: [u8; 16],
}

impl CommandBlockWrapper {
    pub const LENGTH: usize = 31;

    pub fn new(
        tag: u32,
        transfer_length: u32,
        direction: RequestDirection,
        lun: u8,
        cdb: &[u8],
    ) -> Self {
        let mut cbw_cb = [0u8; 16];
        let copy_len = cdb.len().min(16);
        cbw_cb[..copy_len].copy_from_slice(&cdb[..copy_len]);

        let flags = match direction {
            RequestDirection::DeviceToHost => 0x80,
            RequestDirection::HostToDevice => 0x00,
        };

        Self {
            d_cbw_signature: CBW_SIGNATURE,
            d_cbw_tag: tag,
            d_cbw_data_transfer_length: transfer_length,
            bm_cbw_flags: flags,
            b_cbw_lun: lun & 0x0F,
            b_cbw_cb_length: copy_len as u8,
            cbw_cb,
        }
    }

    pub fn to_bytes(&self) -> [u8; 31] {
        let mut buf = [0u8; 31];
        buf[0..4].copy_from_slice(&self.d_cbw_signature.to_le_bytes());
        buf[4..8].copy_from_slice(&self.d_cbw_tag.to_le_bytes());
        buf[8..12].copy_from_slice(&self.d_cbw_data_transfer_length.to_le_bytes());
        buf[12] = self.bm_cbw_flags;
        buf[13] = self.b_cbw_lun;
        buf[14] = self.b_cbw_cb_length;
        buf[15..31].copy_from_slice(&self.cbw_cb);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        let sig = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if sig != CBW_SIGNATURE {
            return Err(UsbError::InvalidDescriptor);
        }
        let tag = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let xfer_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let flags = data[12];
        let lun = data[13];
        let cb_len = data[14];
        let mut cbw_cb = [0u8; 16];
        cbw_cb.copy_from_slice(&data[15..31]);

        Ok(Self {
            d_cbw_signature: sig,
            d_cbw_tag: tag,
            d_cbw_data_transfer_length: xfer_len,
            bm_cbw_flags: flags,
            b_cbw_lun: lun,
            b_cbw_cb_length: cb_len,
            cbw_cb,
        })
    }
}

/// Standard 13-byte Command Status Wrapper (CSW).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStatusWrapper {
    pub d_csw_signature: u32,
    pub d_csw_tag: u32,
    pub d_csw_data_residue: u32,
    pub b_csw_status: u8,
}

impl CommandStatusWrapper {
    pub const LENGTH: usize = 13;

    pub fn new(tag: u32, residue: u32, status: u8) -> Self {
        Self {
            d_csw_signature: CSW_SIGNATURE,
            d_csw_tag: tag,
            d_csw_data_residue: residue,
            b_csw_status: status,
        }
    }

    pub fn to_bytes(&self) -> [u8; 13] {
        let mut buf = [0u8; 13];
        buf[0..4].copy_from_slice(&self.d_csw_signature.to_le_bytes());
        buf[4..8].copy_from_slice(&self.d_csw_tag.to_le_bytes());
        buf[8..12].copy_from_slice(&self.d_csw_data_residue.to_le_bytes());
        buf[12] = self.b_csw_status;
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        let sig = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if sig != CSW_SIGNATURE {
            return Err(UsbError::InvalidDescriptor);
        }
        let tag = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let residue = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let status = data[12];

        Ok(Self {
            d_csw_signature: sig,
            d_csw_tag: tag,
            d_csw_data_residue: residue,
            b_csw_status: status,
        })
    }

    pub fn is_success(&self) -> bool {
        self.b_csw_status == CSW_STATUS_PASSED
    }
}
