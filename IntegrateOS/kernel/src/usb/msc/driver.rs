//! Integrate OS — USB Mass Storage Device Driver
//!
//! Provides the concrete `StorageDriver` block device implementation interfacing
//! with USB flash drives and external hard drives via Bulk-Only Transport (BOT).

use alloc::string::String;
use crate::drivers::device::{DeviceCategory, Driver, StorageDriver};
use crate::drivers::traits::HardwareSupportStatus;
use super::super::controller::UsbHostController;
use super::super::types::*;
use super::bot::*;
use super::scsi::*;

/// An active USB Mass Storage Block Device.
pub struct UsbStorageDevice {
    pub dev_address: u8,
    pub controller_id: usize,
    pub ep_in: u8,
    pub ep_out: u8,
    pub lun: u8,
    tag_counter: u32,
    pub total_sectors: u64,
    pub sector_size: usize,
    pub vendor: String,
    pub product: String,
    pub revision: String,
}

impl UsbStorageDevice {
    pub fn new(
        dev_address: u8,
        controller_id: usize,
        ep_in: u8,
        ep_out: u8,
        lun: u8,
    ) -> Self {
        Self {
            dev_address,
            controller_id,
            ep_in,
            ep_out,
            lun,
            tag_counter: 1,
            total_sectors: 0,
            sector_size: 512,
            vendor: String::from("USB"),
            product: String::from("Storage Device"),
            revision: String::from("1.0"),
        }
    }

    /// Next unique BOT transfer tag.
    fn next_tag(&mut self) -> u32 {
        let tag = self.tag_counter;
        self.tag_counter = self.tag_counter.wrapping_add(1);
        tag
    }

    /// Performs a full BOT transaction using an explicit controller reference.
    pub fn exec_bot_with_ctrl(
        &mut self,
        ctrl: &mut dyn UsbHostController,
        cdb: &[u8],
        dir: RequestDirection,
        data: Option<&mut [u8]>,
    ) -> Result<(), UsbError> {
        let tag = self.next_tag();
        let transfer_len = data.as_ref().map_or(0, |d| d.len() as u32);

        // 1. Build and transmit 31-byte CBW
        let cbw = CommandBlockWrapper::new(tag, transfer_len, dir, self.lun, cdb);
        let mut cbw_bytes = cbw.to_bytes();

        let addr = self.dev_address;
        let ep_out = self.ep_out;
        let ep_in = self.ep_in;

        ctrl.bulk_transfer(addr, ep_out, RequestDirection::HostToDevice, &mut cbw_bytes)?;

        // 2. Transfer data payload if present
        if let Some(buf) = data {
            if !buf.is_empty() {
                let ep = match dir {
                    RequestDirection::HostToDevice => ep_out,
                    RequestDirection::DeviceToHost => ep_in,
                };
                ctrl.bulk_transfer(addr, ep, dir, buf)?;
            }
        }

        // 3. Receive 13-byte CSW on Bulk IN endpoint
        let mut csw_bytes = [0u8; 13];
        ctrl.bulk_transfer(addr, ep_in, RequestDirection::DeviceToHost, &mut csw_bytes)?;

        let csw = CommandStatusWrapper::from_bytes(&csw_bytes)?;
        if !csw.is_success() {
            return Err(UsbError::PhaseError);
        }

        Ok(())
    }

    /// Performs a full BOT Command Block Wrapper (CBW) -> Data -> Status (CSW) transaction.
    pub fn exec_bot(
        &mut self,
        cdb: &[u8],
        dir: RequestDirection,
        data: Option<&mut [u8]>,
    ) -> Result<(), UsbError> {
        let controller_id = self.controller_id;
        crate::usb::manager::with_controller_mut(controller_id, |ctrl| {
            self.exec_bot_with_ctrl(ctrl, cdb, dir, data)
        })?
    }

    /// Queries device identity via SCSI `INQUIRY` with provided controller reference.
    pub fn query_inquiry_with_ctrl(
        &mut self,
        ctrl: &mut dyn UsbHostController,
    ) -> Result<ScsiInquiryData, UsbError> {
        let cdb = build_inquiry_cdb(36);
        let mut data_buf = [0u8; 36];
        self.exec_bot_with_ctrl(ctrl, &cdb, RequestDirection::DeviceToHost, Some(&mut data_buf))?;

        let inquiry = ScsiInquiryData::parse(&data_buf)?;
        self.vendor = inquiry.vendor_id.clone();
        self.product = inquiry.product_id.clone();
        self.revision = inquiry.revision.clone();
        Ok(inquiry)
    }

    /// Queries device identity via SCSI `INQUIRY`.
    pub fn query_inquiry(&mut self) -> Result<ScsiInquiryData, UsbError> {
        let controller_id = self.controller_id;
        crate::usb::manager::with_controller_mut(controller_id, |ctrl| {
            self.query_inquiry_with_ctrl(ctrl)
        })?
    }

    /// Queries device capacity via SCSI `READ CAPACITY (10)` with provided controller reference.
    pub fn query_capacity_with_ctrl(
        &mut self,
        ctrl: &mut dyn UsbHostController,
    ) -> Result<ScsiCapacityData, UsbError> {
        let cdb = build_read_capacity_cdb();
        let mut data_buf = [0u8; 8];
        self.exec_bot_with_ctrl(ctrl, &cdb, RequestDirection::DeviceToHost, Some(&mut data_buf))?;

        let cap = ScsiCapacityData::parse(&data_buf)?;
        self.total_sectors = (cap.last_lba as u64) + 1;
        self.sector_size = cap.block_size as usize;
        Ok(cap)
    }

    /// Queries device capacity via SCSI `READ CAPACITY (10)`.
    pub fn query_capacity(&mut self) -> Result<ScsiCapacityData, UsbError> {
        let controller_id = self.controller_id;
        crate::usb::manager::with_controller_mut(controller_id, |ctrl| {
            self.query_capacity_with_ctrl(ctrl)
        })?
    }
}

impl Driver for UsbStorageDevice {
    fn name(&self) -> &str {
        "USB Mass Storage Device"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Storage
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        self.query_inquiry().map_err(|_| "SCSI Inquiry failed")?;
        self.query_capacity().map_err(|_| "SCSI Read Capacity failed")?;
        Ok(())
    }
}

impl StorageDriver for UsbStorageDevice {
    fn block_size(&self) -> usize {
        self.sector_size
    }

    fn total_blocks(&self) -> u64 {
        self.total_sectors
    }

    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        if buf.len() < self.sector_size {
            return Err("Buffer too small for sector read");
        }
        let cdb = build_read10_cdb(block as u32, 1);
        self.exec_bot(&cdb, RequestDirection::DeviceToHost, Some(&mut buf[..self.sector_size]))
            .map_err(|_| "USB BOT Read(10) failed")
    }

    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<(), &'static str> {
        if buf.len() < self.sector_size {
            return Err("Buffer too small for sector write");
        }
        let cdb = build_write10_cdb(block as u32, 1);
        let mut write_data = alloc::vec![0u8; self.sector_size];
        write_data.copy_from_slice(&buf[..self.sector_size]);
        self.exec_bot(&cdb, RequestDirection::HostToDevice, Some(&mut write_data))
            .map_err(|_| "USB BOT Write(10) failed")
    }
}
