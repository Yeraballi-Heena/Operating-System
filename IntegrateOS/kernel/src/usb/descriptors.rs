//! Integrate OS — USB Descriptor Serialization & Parsing
//!
//! Provides standard representations for:
//! - Device Descriptors (18 bytes)
//! - Configuration Descriptors (9 bytes)
//! - Interface Descriptors (9 bytes)
//! - Endpoint Descriptors (7 bytes)
//! - HID Class Descriptors (9 bytes)
//! - String Descriptors (UTF-16LE to Rust String)
//! - Hierarchical configuration tree parser

use alloc::string::String;
use alloc::vec::Vec;
use super::types::*;

/// Standard 18-byte USB Device Descriptor (Table 9-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbDeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_subclass: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

impl UsbDeviceDescriptor {
    pub const LENGTH: usize = 18;

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 8 {
            return Err(UsbError::BufferTooSmall);
        }
        if data[0] != 18 || data[1] != DESC_DEVICE {
            return Err(UsbError::InvalidDescriptor);
        }

        let bcd_usb = if data.len() >= 4 { u16::from_le_bytes([data[2], data[3]]) } else { 0 };
        let b_device_class = if data.len() >= 5 { data[4] } else { 0 };
        let b_device_subclass = if data.len() >= 6 { data[5] } else { 0 };
        let b_device_protocol = if data.len() >= 7 { data[6] } else { 0 };
        let b_max_packet_size0 = data[7];
        let id_vendor = if data.len() >= 10 { u16::from_le_bytes([data[8], data[9]]) } else { 0 };
        let id_product = if data.len() >= 12 { u16::from_le_bytes([data[10], data[11]]) } else { 0 };
        let bcd_device = if data.len() >= 14 { u16::from_le_bytes([data[12], data[13]]) } else { 0 };
        let i_manufacturer = if data.len() >= 15 { data[14] } else { 0 };
        let i_product = if data.len() >= 16 { data[15] } else { 0 };
        let i_serial_number = if data.len() >= 17 { data[16] } else { 0 };
        let b_num_configurations = if data.len() >= 18 { data[17] } else { 0 };

        Ok(Self {
            b_length: data[0],
            b_descriptor_type: data[1],
            bcd_usb,
            b_device_class,
            b_device_subclass,
            b_device_protocol,
            b_max_packet_size0,
            id_vendor,
            id_product,
            bcd_device,
            i_manufacturer,
            i_product,
            i_serial_number,
            b_num_configurations,
        })
    }

    pub fn to_bytes(&self) -> [u8; 18] {
        let bcd_usb = self.bcd_usb.to_le_bytes();
        let id_vendor = self.id_vendor.to_le_bytes();
        let id_product = self.id_product.to_le_bytes();
        let bcd_device = self.bcd_device.to_le_bytes();

        [
            self.b_length,
            self.b_descriptor_type,
            bcd_usb[0],
            bcd_usb[1],
            self.b_device_class,
            self.b_device_subclass,
            self.b_device_protocol,
            self.b_max_packet_size0,
            id_vendor[0],
            id_vendor[1],
            id_product[0],
            id_product[1],
            bcd_device[0],
            bcd_device[1],
            self.i_manufacturer,
            self.i_product,
            self.i_serial_number,
            self.b_num_configurations,
        ]
    }
}

/// Standard 9-byte USB Configuration Descriptor (Table 9-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbConfigurationDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub b_max_power: u8,
}

impl UsbConfigurationDescriptor {
    pub const LENGTH: usize = 9;

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        if data[0] != 9 || data[1] != DESC_CONFIGURATION {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            b_length: data[0],
            b_descriptor_type: data[1],
            w_total_length: u16::from_le_bytes([data[2], data[3]]),
            b_num_interfaces: data[4],
            b_configuration_value: data[5],
            i_configuration: data[6],
            bm_attributes: data[7],
            b_max_power: data[8],
        })
    }

    pub fn to_bytes(&self) -> [u8; 9] {
        let total_len = self.w_total_length.to_le_bytes();
        [
            self.b_length,
            self.b_descriptor_type,
            total_len[0],
            total_len[1],
            self.b_num_interfaces,
            self.b_configuration_value,
            self.i_configuration,
            self.bm_attributes,
            self.b_max_power,
        ]
    }

    pub fn is_self_powered(&self) -> bool {
        (self.bm_attributes & 0x40) != 0
    }

    pub fn supports_remote_wakeup(&self) -> bool {
        (self.bm_attributes & 0x20) != 0
    }

    /// Max power consumption in milliamps (2 mA increments).
    pub fn max_power_ma(&self) -> u16 {
        (self.b_max_power as u16) * 2
    }
}

/// Standard 9-byte USB Interface Descriptor (Table 9-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbInterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_interface_number: u8,
    pub b_alternate_setting: u8,
    pub b_num_endpoints: u8,
    pub b_interface_class: u8,
    pub b_interface_subclass: u8,
    pub b_interface_protocol: u8,
    pub i_interface: u8,
}

impl UsbInterfaceDescriptor {
    pub const LENGTH: usize = 9;

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        if data[0] != 9 || data[1] != DESC_INTERFACE {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            b_length: data[0],
            b_descriptor_type: data[1],
            b_interface_number: data[2],
            b_alternate_setting: data[3],
            b_num_endpoints: data[4],
            b_interface_class: data[5],
            b_interface_subclass: data[6],
            b_interface_protocol: data[7],
            i_interface: data[8],
        })
    }

    pub fn to_bytes(&self) -> [u8; 9] {
        [
            self.b_length,
            self.b_descriptor_type,
            self.b_interface_number,
            self.b_alternate_setting,
            self.b_num_endpoints,
            self.b_interface_class,
            self.b_interface_subclass,
            self.b_interface_protocol,
            self.i_interface,
        ]
    }
}

/// Standard 7-byte USB Endpoint Descriptor (Table 9-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbEndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub b_endpoint_address: u8,
    pub bm_attributes: u8,
    pub w_max_packet_size: u16,
    pub b_interval: u8,
}

impl UsbEndpointDescriptor {
    pub const LENGTH: usize = 7;

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        if data[0] != 7 || data[1] != DESC_ENDPOINT {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            b_length: data[0],
            b_descriptor_type: data[1],
            b_endpoint_address: data[2],
            bm_attributes: data[3],
            w_max_packet_size: u16::from_le_bytes([data[4], data[5]]),
            b_interval: data[6],
        })
    }

    pub fn to_bytes(&self) -> [u8; 7] {
        let max_packet = self.w_max_packet_size.to_le_bytes();
        [
            self.b_length,
            self.b_descriptor_type,
            self.b_endpoint_address,
            self.bm_attributes,
            max_packet[0],
            max_packet[1],
            self.b_interval,
        ]
    }

    /// Extract endpoint number (0..15).
    pub fn endpoint_number(&self) -> u8 {
        self.b_endpoint_address & 0x0F
    }

    /// Extract endpoint transfer direction.
    pub fn direction(&self) -> RequestDirection {
        RequestDirection::from_u8(self.b_endpoint_address)
    }

    /// Extract endpoint transfer type (Control, Isochronous, Bulk, Interrupt).
    pub fn transfer_type(&self) -> UsbTransferType {
        UsbTransferType::from_bm_attributes(self.bm_attributes)
    }
}

/// Standard 9-byte USB HID Class Descriptor (HID Spec 1.11, Section 6.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbHidDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: u8,
    pub bcd_hid: u16,
    pub b_country_code: u8,
    pub b_num_descriptors: u8,
    pub b_report_descriptor_type: u8,
    pub w_report_descriptor_length: u16,
}

impl UsbHidDescriptor {
    pub const LENGTH: usize = 9;

    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < Self::LENGTH {
            return Err(UsbError::BufferTooSmall);
        }
        if data[1] != DESC_HID {
            return Err(UsbError::InvalidDescriptor);
        }

        Ok(Self {
            b_length: data[0],
            b_descriptor_type: data[1],
            bcd_hid: u16::from_le_bytes([data[2], data[3]]),
            b_country_code: data[4],
            b_num_descriptors: data[5],
            b_report_descriptor_type: data[6],
            w_report_descriptor_length: u16::from_le_bytes([data[7], data[8]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 9] {
        let bcd_hid = self.bcd_hid.to_le_bytes();
        let report_len = self.w_report_descriptor_length.to_le_bytes();
        [
            self.b_length,
            self.b_descriptor_type,
            bcd_hid[0],
            bcd_hid[1],
            self.b_country_code,
            self.b_num_descriptors,
            self.b_report_descriptor_type,
            report_len[0],
            report_len[1],
        ]
    }
}

/// Helper for decoding USB String Descriptors (UTF-16LE characters into ASCII/UTF-8 String).
pub fn parse_string_descriptor(data: &[u8]) -> Result<String, UsbError> {
    if data.len() < 2 {
        return Err(UsbError::BufferTooSmall);
    }
    let length = data[0] as usize;
    if length > data.len() || data[1] != DESC_STRING {
        return Err(UsbError::InvalidDescriptor);
    }

    let mut result = String::new();
    let mut offset = 2;
    while offset + 1 < length {
        let code_point = u16::from_le_bytes([data[offset], data[offset + 1]]);
        if let Some(ch) = core::char::from_u32(code_point as u32) {
            result.push(ch);
        } else {
            result.push('?');
        }
        offset += 2;
    }

    Ok(result)
}

/// An interface descriptor together with its optional HID descriptor and attached endpoints.
#[derive(Debug, Clone)]
pub struct ParsedInterface {
    pub interface: UsbInterfaceDescriptor,
    pub hid: Option<UsbHidDescriptor>,
    pub endpoints: Vec<UsbEndpointDescriptor>,
}

/// Fully parsed configuration tree including all interfaces, endpoints, and class descriptors.
#[derive(Debug, Clone)]
pub struct ParsedConfiguration {
    pub config: UsbConfigurationDescriptor,
    pub interfaces: Vec<ParsedInterface>,
}

/// Parses the full concatenated configuration descriptor returned by `GET_DESCRIPTOR (Configuration)`.
pub fn parse_configuration_tree(data: &[u8]) -> Result<ParsedConfiguration, UsbError> {
    if data.len() < UsbConfigurationDescriptor::LENGTH {
        return Err(UsbError::BufferTooSmall);
    }

    let config = UsbConfigurationDescriptor::from_bytes(&data[..UsbConfigurationDescriptor::LENGTH])?;
    let total_len = config.w_total_length as usize;
    let limit = total_len.min(data.len());

    let mut interfaces = Vec::new();
    let mut current_if: Option<UsbInterfaceDescriptor> = None;
    let mut current_hid: Option<UsbHidDescriptor> = None;
    let mut current_endpoints: Vec<UsbEndpointDescriptor> = Vec::new();

    let mut offset = config.b_length as usize;

    while offset < limit {
        if offset >= data.len() {
            break;
        }
        let desc_len = data[offset] as usize;
        if desc_len == 0 || offset + desc_len > data.len() {
            break;
        }
        let desc_type = data[offset + 1];
        let desc_slice = &data[offset..offset + desc_len];

        match desc_type {
            DESC_INTERFACE => {
                // Save previous interface if active
                if let Some(prev_if) = current_if.take() {
                    interfaces.push(ParsedInterface {
                        interface: prev_if,
                        hid: current_hid.take(),
                        endpoints: core::mem::take(&mut current_endpoints),
                    });
                }
                if let Ok(parsed_if) = UsbInterfaceDescriptor::from_bytes(desc_slice) {
                    current_if = Some(parsed_if);
                }
            }
            DESC_HID => {
                if let Ok(hid_desc) = UsbHidDescriptor::from_bytes(desc_slice) {
                    current_hid = Some(hid_desc);
                }
            }
            DESC_ENDPOINT => {
                if let Ok(ep) = UsbEndpointDescriptor::from_bytes(desc_slice) {
                    current_endpoints.push(ep);
                }
            }
            _ => {
                // Other descriptors (hub, audio, security, etc.) are skipped gracefully
            }
        }

        offset += desc_len;
    }

    // Flush final interface
    if let Some(prev_if) = current_if.take() {
        interfaces.push(ParsedInterface {
            interface: prev_if,
            hid: current_hid.take(),
            endpoints: current_endpoints,
        });
    }

    Ok(ParsedConfiguration { config, interfaces })
}
