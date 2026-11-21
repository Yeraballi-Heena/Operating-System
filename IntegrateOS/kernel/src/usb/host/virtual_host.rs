//! Integrate OS — Virtual USB Host Controller & Root Hub Simulation
//!
//! Provides a software-emulated USB Host Controller implementing `UsbHostController`.
//! Manages simulated USB ports, answers standard USB control transfers, serves HID
//! interrupt transfers, and drives Bulk-Only Transport (BOT) SCSI block operations.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use crate::pci::PciAddress;
use super::super::controller::{UsbControllerType, UsbHostController};
use super::super::descriptors::*;
use super::super::setup::UsbSetupPacket;
use super::super::types::*;

/// A simulated USB peripheral attached to a virtual port.
pub struct SimulatedUsbDevice {
    pub default_address: u8,
    pub assigned_address: u8,
    pub speed: UsbSpeed,
    pub device_desc: UsbDeviceDescriptor,
    pub config_data: Vec<u8>,
    pub manufacturer_str: String,
    pub product_str: String,
    pub serial_str: String,
    pub active_configuration: u8,
    pub pending_keyboard_reports: VecDeque<[u8; 8]>,
    pub pending_mouse_reports: VecDeque<[u8; 4]>,
    pub storage_blocks: Vec<u8>, // 512-byte sectors for simulated USB flash drive
    pub storage_capacity_blocks: u32,
    pub last_cbw_tag: u32,
    pub last_cbw_lun: u8,
    pub pending_lba: usize,
    pub pending_sector_count: usize,
}

impl SimulatedUsbDevice {
    /// Creates a simulated USB HID Boot Keyboard.
    pub fn new_keyboard() -> Self {
        let device_desc = UsbDeviceDescriptor {
            b_length: 18,
            b_descriptor_type: DESC_DEVICE,
            bcd_usb: 0x0200, // USB 2.0
            b_device_class: CLASS_PER_INTERFACE,
            b_device_subclass: 0,
            b_device_protocol: 0,
            b_max_packet_size0: 64,
            id_vendor: 0x1234,
            id_product: 0x0001,
            bcd_device: 0x0100,
            i_manufacturer: 1,
            i_product: 2,
            i_serial_number: 3,
            b_num_configurations: 1,
        };

        // Construct 9-byte config + 9-byte interface + 9-byte HID + 7-byte interrupt endpoint
        let mut config_data = Vec::new();
        let total_len: u16 = 9 + 9 + 9 + 7;

        let config_desc = UsbConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_CONFIGURATION,
            w_total_length: total_len,
            b_num_interfaces: 1,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0xA0, // Bus-powered, remote wakeup
            b_max_power: 50,     // 100 mA
        };
        config_data.extend_from_slice(&config_desc.to_bytes());

        let if_desc = UsbInterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_INTERFACE,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: CLASS_HID,
            b_interface_subclass: HID_BOOT_SUBCLASS,
            b_interface_protocol: HID_KEYBOARD_PROTOCOL,
            i_interface: 0,
        };
        config_data.extend_from_slice(&if_desc.to_bytes());

        let hid_desc = UsbHidDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_HID,
            bcd_hid: 0x0111,
            b_country_code: 0,
            b_num_descriptors: 1,
            b_report_descriptor_type: DESC_HID_REPORT,
            w_report_descriptor_length: 63,
        };
        config_data.extend_from_slice(&hid_desc.to_bytes());

        let ep_desc = UsbEndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DESC_ENDPOINT,
            b_endpoint_address: 0x81, // IN Endpoint 1
            bm_attributes: 0x03,       // Interrupt
            w_max_packet_size: 8,
            b_interval: 10,
        };
        config_data.extend_from_slice(&ep_desc.to_bytes());

        Self {
            default_address: 0,
            assigned_address: 0,
            speed: UsbSpeed::FullSpeed,
            device_desc,
            config_data,
            manufacturer_str: String::from("Integrate OS"),
            product_str: String::from("Virtual USB Keyboard"),
            serial_str: String::from("KBD-001"),
            active_configuration: 0,
            pending_keyboard_reports: VecDeque::new(),
            pending_mouse_reports: VecDeque::new(),
            storage_blocks: Vec::new(),
            storage_capacity_blocks: 0,
            last_cbw_tag: 0,
            last_cbw_lun: 0,
            pending_lba: 0,
            pending_sector_count: 0,
        }
    }

    /// Creates a simulated USB HID Boot Mouse.
    pub fn new_mouse() -> Self {
        let device_desc = UsbDeviceDescriptor {
            b_length: 18,
            b_descriptor_type: DESC_DEVICE,
            bcd_usb: 0x0200,
            b_device_class: CLASS_PER_INTERFACE,
            b_device_subclass: 0,
            b_device_protocol: 0,
            b_max_packet_size0: 64,
            id_vendor: 0x1234,
            id_product: 0x0002,
            bcd_device: 0x0100,
            i_manufacturer: 1,
            i_product: 2,
            i_serial_number: 3,
            b_num_configurations: 1,
        };

        let mut config_data = Vec::new();
        let total_len: u16 = 9 + 9 + 9 + 7;

        let config_desc = UsbConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_CONFIGURATION,
            w_total_length: total_len,
            b_num_interfaces: 1,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0xA0,
            b_max_power: 50,
        };
        config_data.extend_from_slice(&config_desc.to_bytes());

        let if_desc = UsbInterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_INTERFACE,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: CLASS_HID,
            b_interface_subclass: HID_BOOT_SUBCLASS,
            b_interface_protocol: HID_MOUSE_PROTOCOL,
            i_interface: 0,
        };
        config_data.extend_from_slice(&if_desc.to_bytes());

        let hid_desc = UsbHidDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_HID,
            bcd_hid: 0x0111,
            b_country_code: 0,
            b_num_descriptors: 1,
            b_report_descriptor_type: DESC_HID_REPORT,
            w_report_descriptor_length: 52,
        };
        config_data.extend_from_slice(&hid_desc.to_bytes());

        let ep_desc = UsbEndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DESC_ENDPOINT,
            b_endpoint_address: 0x82, // IN Endpoint 2
            bm_attributes: 0x03,       // Interrupt
            w_max_packet_size: 4,
            b_interval: 10,
        };
        config_data.extend_from_slice(&ep_desc.to_bytes());

        Self {
            default_address: 0,
            assigned_address: 0,
            speed: UsbSpeed::FullSpeed,
            device_desc,
            config_data,
            manufacturer_str: String::from("Integrate OS"),
            product_str: String::from("Virtual USB Optical Mouse"),
            serial_str: String::from("MOU-002"),
            active_configuration: 0,
            pending_keyboard_reports: VecDeque::new(),
            pending_mouse_reports: VecDeque::new(),
            storage_blocks: Vec::new(),
            storage_capacity_blocks: 0,
            last_cbw_tag: 0,
            last_cbw_lun: 0,
            pending_lba: 0,
            pending_sector_count: 0,
        }
    }

    /// Creates a simulated USB Mass Storage Flash Drive (BOT + SCSI).
    pub fn new_storage(capacity_sectors: u32) -> Self {
        let device_desc = UsbDeviceDescriptor {
            b_length: 18,
            b_descriptor_type: DESC_DEVICE,
            bcd_usb: 0x0200,
            b_device_class: CLASS_PER_INTERFACE,
            b_device_subclass: 0,
            b_device_protocol: 0,
            b_max_packet_size0: 64,
            id_vendor: 0xABCD,
            id_product: 0x1234,
            bcd_device: 0x0100,
            i_manufacturer: 1,
            i_product: 2,
            i_serial_number: 3,
            b_num_configurations: 1,
        };

        let mut config_data = Vec::new();
        let total_len: u16 = 9 + 9 + 7 + 7;

        let config_desc = UsbConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_CONFIGURATION,
            w_total_length: total_len,
            b_num_interfaces: 1,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0x80, // Bus-powered
            b_max_power: 100,    // 200 mA
        };
        config_data.extend_from_slice(&config_desc.to_bytes());

        let if_desc = UsbInterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DESC_INTERFACE,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 2,
            b_interface_class: CLASS_MASS_STORAGE,
            b_interface_subclass: MSC_SUBCLASS_SCSI,
            b_interface_protocol: MSC_PROTOCOL_BOT,
            i_interface: 0,
        };
        config_data.extend_from_slice(&if_desc.to_bytes());

        let ep_out = UsbEndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DESC_ENDPOINT,
            b_endpoint_address: 0x02, // OUT Endpoint 2 (Bulk)
            bm_attributes: 0x02,       // Bulk
            w_max_packet_size: 512,
            b_interval: 0,
        };
        config_data.extend_from_slice(&ep_out.to_bytes());

        let ep_in = UsbEndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DESC_ENDPOINT,
            b_endpoint_address: 0x81, // IN Endpoint 1 (Bulk)
            bm_attributes: 0x02,       // Bulk
            w_max_packet_size: 512,
            b_interval: 0,
        };
        config_data.extend_from_slice(&ep_in.to_bytes());

        let total_bytes = (capacity_sectors as usize) * 512;
        let mut storage_blocks = vec![0u8; total_bytes];

        // Seed sector 0 with a valid MBR / boot record signature
        if total_bytes >= 512 {
            storage_blocks[510] = 0x55;
            storage_blocks[511] = 0xAA;
        }

        Self {
            default_address: 0,
            assigned_address: 0,
            speed: UsbSpeed::HighSpeed,
            device_desc,
            config_data,
            manufacturer_str: String::from("Integrate OS"),
            product_str: String::from("Virtual Flash Drive (16MB)"),
            serial_str: String::from("MSC-8899"),
            active_configuration: 0,
            pending_keyboard_reports: VecDeque::new(),
            pending_mouse_reports: VecDeque::new(),
            storage_blocks,
            storage_capacity_blocks: capacity_sectors,
            last_cbw_tag: 0,
            last_cbw_lun: 0,
            pending_lba: 0,
            pending_sector_count: 0,
        }
    }

    /// Encodes a string into standard UTF-16LE String Descriptor bytes.
    pub fn build_string_descriptor(s: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        let len = (s.len() * 2 + 2) as u8;
        buf.push(len);
        buf.push(DESC_STRING);
        for ch in s.chars() {
            let u16_val = ch as u16;
            buf.push((u16_val & 0xFF) as u8);
            buf.push(((u16_val >> 8) & 0xFF) as u8);
        }
        buf
    }
}

/// Virtual USB Host Controller with pluggable virtual devices.
pub struct VirtualUsbHostController {
    ports: Vec<Option<SimulatedUsbDevice>>,
}

impl VirtualUsbHostController {
    pub fn new() -> Self {
        let mut ports = Vec::new();
        // Port 0: Keyboard
        ports.push(Some(SimulatedUsbDevice::new_keyboard()));
        // Port 1: Mouse
        ports.push(Some(SimulatedUsbDevice::new_mouse()));
        // Port 2: Mass Storage (32768 sectors * 512 bytes = 16 MiB)
        ports.push(Some(SimulatedUsbDevice::new_storage(32768)));
        // Port 3: Empty port
        ports.push(None);

        Self { ports }
    }

    /// Queues an 8-byte input report into the keyboard on port 0.
    pub fn queue_keyboard_report(&mut self, report: [u8; 8]) {
        if let Some(Some(ref mut dev)) = self.ports.get_mut(0) {
            dev.pending_keyboard_reports.push_back(report);
        }
    }

    /// Queues a 4-byte input report into the mouse on port 1.
    pub fn queue_mouse_report(&mut self, report: [u8; 4]) {
        if let Some(Some(ref mut dev)) = self.ports.get_mut(1) {
            dev.pending_mouse_reports.push_back(report);
        }
    }

    /// Finds device matching either default address 0 or assigned bus address.
    fn find_device_mut(&mut self, addr: u8) -> Option<&mut SimulatedUsbDevice> {
        for slot in self.ports.iter_mut() {
            if let Some(ref mut dev) = slot {
                if dev.assigned_address == addr || (dev.assigned_address == 0 && addr == 0) {
                    return Some(dev);
                }
            }
        }
        None
    }
}

impl UsbHostController for VirtualUsbHostController {
    fn name(&self) -> &'static str {
        "Virtual USB 2.0 Host Controller & Root Hub"
    }

    fn controller_type(&self) -> UsbControllerType {
        UsbControllerType::Virtual
    }

    fn pci_address(&self) -> Option<PciAddress> {
        None
    }

    fn init(&mut self) -> Result<(), UsbError> {
        Ok(())
    }

    fn poll(&mut self) {}

    fn port_count(&self) -> u8 {
        self.ports.len() as u8
    }

    fn is_port_connected(&self, port: u8) -> bool {
        if let Some(slot) = self.ports.get(port as usize) {
            slot.is_some()
        } else {
            false
        }
    }

    fn reset_port(&mut self, port: u8) -> Result<(), UsbError> {
        if let Some(Some(ref mut dev)) = self.ports.get_mut(port as usize) {
            dev.assigned_address = 0;
            dev.active_configuration = 0;
            Ok(())
        } else {
            Err(UsbError::DeviceNotFound)
        }
    }

    fn control_transfer(
        &mut self,
        addr: u8,
        setup: &UsbSetupPacket,
        mut data: Option<&mut [u8]>,
    ) -> Result<usize, UsbError> {
        let dev = self.find_device_mut(addr).ok_or(UsbError::DeviceNotFound)?;

        match setup.b_request {
            REQ_GET_DESCRIPTOR => {
                let desc_type = (setup.w_value >> 8) as u8;
                let desc_index = (setup.w_value & 0xFF) as u8;
                let out_slice = data.as_deref_mut().ok_or(UsbError::BufferTooSmall)?;

                match desc_type {
                    DESC_DEVICE => {
                        let bytes = dev.device_desc.to_bytes();
                        let copy_len = bytes.len().min(setup.w_length as usize).min(out_slice.len());
                        out_slice[..copy_len].copy_from_slice(&bytes[..copy_len]);
                        Ok(copy_len)
                    }
                    DESC_CONFIGURATION => {
                        let copy_len = dev.config_data.len().min(setup.w_length as usize).min(out_slice.len());
                        out_slice[..copy_len].copy_from_slice(&dev.config_data[..copy_len]);
                        Ok(copy_len)
                    }
                    DESC_STRING => {
                        let str_bytes = match desc_index {
                            0 => vec![4, DESC_STRING, 0x09, 0x04], // Language ID: US English (0x0409)
                            1 => SimulatedUsbDevice::build_string_descriptor(&dev.manufacturer_str),
                            2 => SimulatedUsbDevice::build_string_descriptor(&dev.product_str),
                            3 => SimulatedUsbDevice::build_string_descriptor(&dev.serial_str),
                            _ => vec![2, DESC_STRING],
                        };
                        let copy_len = str_bytes.len().min(setup.w_length as usize).min(out_slice.len());
                        out_slice[..copy_len].copy_from_slice(&str_bytes[..copy_len]);
                        Ok(copy_len)
                    }
                    _ => Err(UsbError::InvalidDescriptor),
                }
            }
            REQ_SET_ADDRESS => {
                dev.assigned_address = setup.w_value as u8;
                Ok(0)
            }
            REQ_SET_CONFIGURATION => {
                dev.active_configuration = setup.w_value as u8;
                Ok(0)
            }
            REQ_GET_CONFIGURATION => {
                if let Some(buf) = data {
                    if !buf.is_empty() {
                        buf[0] = dev.active_configuration;
                        return Ok(1);
                    }
                }
                Err(UsbError::BufferTooSmall)
            }
            HID_REQ_SET_PROTOCOL | HID_REQ_SET_IDLE => {
                // Acknowledge HID control settings
                Ok(0)
            }
            MSC_BOT_RESET => {
                Ok(0)
            }
            MSC_BOT_GET_MAX_LUN => {
                if let Some(buf) = data {
                    if !buf.is_empty() {
                        buf[0] = 0; // Single Logical Unit Number (LUN 0)
                        return Ok(1);
                    }
                }
                Err(UsbError::BufferTooSmall)
            }
            _ => Ok(0),
        }
    }

    fn bulk_transfer(
        &mut self,
        addr: u8,
        endpoint: u8,
        dir: RequestDirection,
        data: &mut [u8],
    ) -> Result<usize, UsbError> {
        let dev = self.find_device_mut(addr).ok_or(UsbError::DeviceNotFound)?;

        match dir {
            RequestDirection::HostToDevice => {
                // Check if this is a 31-byte MSC Command Block Wrapper (CBW)
                if data.len() >= 31 && &data[0..4] == b"USBC" {
                    let tag = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    let lun = data[13] & 0x0F;
                    dev.last_cbw_tag = tag;
                    dev.last_cbw_lun = lun;

                    // Inspect SCSI command (byte 15 is CDB opcode)
                    let scsi_opcode = data[15];
                    if scsi_opcode == 0x2A || scsi_opcode == 0x28 {
                        // WRITE (10) or READ (10) command
                        let lba = u32::from_be_bytes([data[17], data[18], data[19], data[20]]) as usize;
                        let sectors = u16::from_be_bytes([data[22], data[23]]) as usize;
                        dev.pending_lba = lba;
                        dev.pending_sector_count = sectors;
                    }
                    return Ok(31);
                } else if endpoint == 0x02 && !dev.storage_blocks.is_empty() {
                    // Raw data write into storage at pending_lba
                    let offset = dev.pending_lba * 512;
                    if offset + data.len() <= dev.storage_blocks.len() {
                        dev.storage_blocks[offset..offset + data.len()].copy_from_slice(data);
                    }
                    return Ok(data.len());
                }
                Ok(data.len())
            }
            RequestDirection::DeviceToHost => {
                // MSC Command Status Wrapper (CSW, 13 bytes) or SCSI Inquiry/Read payload
                if data.len() == 13 {
                    // Build 13-byte CSW: "USBS" + Tag + Residue(0) + Status(0 = Passed)
                    data[0..4].copy_from_slice(b"USBS");
                    data[4..8].copy_from_slice(&dev.last_cbw_tag.to_le_bytes());
                    data[8..12].copy_from_slice(&0u32.to_le_bytes()); // 0 data residue
                    data[12] = 0; // Command Passed
                    return Ok(13);
                } else if data.len() == 36 {
                    // SCSI INQUIRY response
                    data.fill(0);
                    data[0] = 0x00; // Direct Access Block Device
                    data[1] = 0x80; // Removable Media
                    data[2] = 0x04; // SPC-2 compliant
                    data[3] = 0x02; // Response data format
                    data[4] = 31;   // Additional length
                    // Vendor: "Integrat"
                    let vendor = b"Integrat";
                    data[8..16].copy_from_slice(vendor);
                    // Product: "Flash Drive     "
                    let prod = b"Flash Drive     ";
                    data[16..32].copy_from_slice(prod);
                    // Revision: "1.00"
                    let rev = b"1.00";
                    data[32..36].copy_from_slice(rev);
                    return Ok(36);
                } else if data.len() == 8 {
                    // SCSI READ CAPACITY (10) response: Last LBA (u32 big-endian) + Block Size (512 big-endian)
                    let last_lba = dev.storage_capacity_blocks.saturating_sub(1);
                    data[0..4].copy_from_slice(&last_lba.to_be_bytes());
                    data[4..8].copy_from_slice(&512u32.to_be_bytes());
                    return Ok(8);
                } else {
                    // Bulk sector read: copy from storage memory at pending_lba
                    let offset = dev.pending_lba * 512;
                    if offset < dev.storage_blocks.len() {
                        let available = dev.storage_blocks.len() - offset;
                        let read_len = data.len().min(available);
                        data[..read_len].copy_from_slice(&dev.storage_blocks[offset..offset + read_len]);
                        return Ok(read_len);
                    } else {
                        data.fill(0);
                        return Ok(data.len());
                    }
                }
            }
        }
    }

    fn interrupt_transfer(
        &mut self,
        addr: u8,
        _endpoint: u8,
        dir: RequestDirection,
        data: &mut [u8],
    ) -> Result<usize, UsbError> {
        let dev = self.find_device_mut(addr).ok_or(UsbError::DeviceNotFound)?;

        if dir == RequestDirection::DeviceToHost {
            if let Some(report) = dev.pending_keyboard_reports.pop_front() {
                let copy_len = report.len().min(data.len());
                data[..copy_len].copy_from_slice(&report[..copy_len]);
                return Ok(copy_len);
            }
            if let Some(report) = dev.pending_mouse_reports.pop_front() {
                let copy_len = report.len().min(data.len());
                data[..copy_len].copy_from_slice(&report[..copy_len]);
                return Ok(copy_len);
            }
        }
        Ok(0)
    }
}
