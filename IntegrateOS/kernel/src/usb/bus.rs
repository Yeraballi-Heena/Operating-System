//! Integrate OS — USB Bus Pipe Transactions
//!
//! Provides high-level request abstractions across standard USB pipes.

use alloc::string::String;
use alloc::vec;
use super::controller::UsbHostController;
use super::descriptors::*;
use super::setup::UsbSetupPacket;
use super::types::*;

/// High-level USB bus transaction helper routines.
pub struct UsbBus;

impl UsbBus {
    /// Reads the standard 18-byte Device Descriptor from a peripheral.
    pub fn get_device_descriptor(
        ctrl: &mut dyn UsbHostController,
        addr: u8,
        max_len: u16,
    ) -> Result<UsbDeviceDescriptor, UsbError> {
        let setup = UsbSetupPacket::get_descriptor(DESC_DEVICE, 0, max_len);
        let mut buf = [0u8; 18];
        let req_len = (max_len as usize).min(18);

        let read = ctrl.control_transfer(addr, &setup, Some(&mut buf[..req_len]))?;
        if read < 8 {
            return Err(UsbError::BufferTooSmall);
        }

        UsbDeviceDescriptor::from_bytes(&buf[..read])
    }

    /// Assigns a unique bus address (1..127) to a device on address 0.
    pub fn set_address(
        ctrl: &mut dyn UsbHostController,
        old_addr: u8,
        new_addr: u8,
    ) -> Result<(), UsbError> {
        let setup = UsbSetupPacket::set_address(new_addr);
        ctrl.control_transfer(old_addr, &setup, None)?;
        Ok(())
    }

    /// Reads and parses the full hierarchical configuration tree.
    pub fn get_configuration(
        ctrl: &mut dyn UsbHostController,
        addr: u8,
        config_index: u8,
    ) -> Result<ParsedConfiguration, UsbError> {
        // 1. Fetch 9-byte header to find total length
        let setup_hdr = UsbSetupPacket::get_descriptor(
            DESC_CONFIGURATION,
            config_index,
            UsbConfigurationDescriptor::LENGTH as u16,
        );
        let mut hdr_buf = [0u8; 9];
        ctrl.control_transfer(addr, &setup_hdr, Some(&mut hdr_buf))?;

        let hdr = UsbConfigurationDescriptor::from_bytes(&hdr_buf)?;
        let total_len = hdr.w_total_length as usize;

        // 2. Fetch full configuration tree
        let setup_full = UsbSetupPacket::get_descriptor(DESC_CONFIGURATION, config_index, total_len as u16);
        let mut full_buf = vec![0u8; total_len];
        ctrl.control_transfer(addr, &setup_full, Some(&mut full_buf))?;

        parse_configuration_tree(&full_buf)
    }

    /// Sets the active device configuration.
    pub fn set_configuration(
        ctrl: &mut dyn UsbHostController,
        addr: u8,
        config_val: u8,
    ) -> Result<(), UsbError> {
        let setup = UsbSetupPacket::set_configuration(config_val);
        ctrl.control_transfer(addr, &setup, None)?;
        Ok(())
    }

    /// Reads and decodes a UTF-16LE String Descriptor.
    pub fn get_string_descriptor(
        ctrl: &mut dyn UsbHostController,
        addr: u8,
        index: u8,
    ) -> Result<String, UsbError> {
        if index == 0 {
            return Ok(String::new());
        }
        let setup = UsbSetupPacket::get_descriptor(DESC_STRING, index, 255);
        let mut buf = vec![0u8; 255];
        let bytes_read = ctrl.control_transfer(addr, &setup, Some(&mut buf))?;
        if bytes_read == 0 {
            return Ok(String::new());
        }
        parse_string_descriptor(&buf[..bytes_read])
    }
}
