//! Integrate OS — USB Setup Packet & Control Transfer Requests
//!
//! Implements the 8-byte setup packet format required by all USB Control endpoint 0 transfers.

use super::types::*;

/// Standard 8-byte USB Setup Data Packet (Table 9-2).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbSetupPacket {
    /// Characteristics of request (Direction, Type, Recipient)
    pub bm_request_type: u8,
    /// Specific request code
    pub b_request: u8,
    /// Word-sized field that varies according to request
    pub w_value: u16,
    /// Word-sized field that varies according to request (often index or offset)
    pub w_index: u16,
    /// Number of bytes to transfer if there is a data stage
    pub w_length: u16,
}

impl UsbSetupPacket {
    /// Constructs a generic USB setup packet.
    pub const fn new(
        direction: RequestDirection,
        req_type: RequestType,
        recipient: RequestRecipient,
        b_request: u8,
        w_value: u16,
        w_index: u16,
        w_length: u16,
    ) -> Self {
        let bm_request_type = direction.to_bit() | req_type.to_bits() | recipient.to_bits();
        Self {
            bm_request_type,
            b_request,
            w_value,
            w_index,
            w_length,
        }
    }

    /// Returns the data direction of the setup transfer.
    pub const fn direction(&self) -> RequestDirection {
        RequestDirection::from_u8(self.bm_request_type)
    }

    /// Returns the type of request (Standard, Class, Vendor).
    pub const fn request_type(&self) -> RequestType {
        RequestType::from_u8(self.bm_request_type)
    }

    /// Returns the designated recipient (Device, Interface, Endpoint).
    pub const fn recipient(&self) -> RequestRecipient {
        RequestRecipient::from_u8(self.bm_request_type)
    }

    /// Serializes the setup packet into an 8-byte array (little-endian byte order).
    pub fn to_bytes(&self) -> [u8; 8] {
        [
            self.bm_request_type,
            self.b_request,
            (self.w_value & 0xFF) as u8,
            ((self.w_value >> 8) & 0xFF) as u8,
            (self.w_index & 0xFF) as u8,
            ((self.w_index >> 8) & 0xFF) as u8,
            (self.w_length & 0xFF) as u8,
            ((self.w_length >> 8) & 0xFF) as u8,
        ]
    }

    /// Parses an 8-byte array into a typed setup packet.
    pub fn from_bytes(data: &[u8]) -> Result<Self, UsbError> {
        if data.len() < 8 {
            return Err(UsbError::BufferTooSmall);
        }
        let bm_request_type = data[0];
        let b_request = data[1];
        let w_value = u16::from_le_bytes([data[2], data[3]]);
        let w_index = u16::from_le_bytes([data[4], data[5]]);
        let w_length = u16::from_le_bytes([data[6], data[7]]);

        Ok(Self {
            bm_request_type,
            b_request,
            w_value,
            w_index,
            w_length,
        })
    }

    // -------------------------------------------------------------------------
    // Standard Request Constructors
    // -------------------------------------------------------------------------

    /// Builds a `GET_DESCRIPTOR` request for reading standard USB descriptors.
    pub const fn get_descriptor(desc_type: u8, desc_index: u8, length: u16) -> Self {
        let w_value = ((desc_type as u16) << 8) | (desc_index as u16);
        Self::new(
            RequestDirection::DeviceToHost,
            RequestType::Standard,
            RequestRecipient::Device,
            REQ_GET_DESCRIPTOR,
            w_value,
            0x0000,
            length,
        )
    }

    /// Builds a `SET_ADDRESS` request assigning a bus address (1..127) to a device.
    pub const fn set_address(address: u8) -> Self {
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Standard,
            RequestRecipient::Device,
            REQ_SET_ADDRESS,
            address as u16,
            0x0000,
            0x0000,
        )
    }

    /// Builds a `GET_CONFIGURATION` request.
    pub const fn get_configuration() -> Self {
        Self::new(
            RequestDirection::DeviceToHost,
            RequestType::Standard,
            RequestRecipient::Device,
            REQ_GET_CONFIGURATION,
            0x0000,
            0x0000,
            0x0001,
        )
    }

    /// Builds a `SET_CONFIGURATION` request activating a specific device configuration.
    pub const fn set_configuration(config_val: u8) -> Self {
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Standard,
            RequestRecipient::Device,
            REQ_SET_CONFIGURATION,
            config_val as u16,
            0x0000,
            0x0000,
        )
    }

    /// Builds a `CLEAR_FEATURE` request to clear endpoint halts (e.g. after STALL).
    pub const fn clear_feature_endpoint(endpoint_addr: u8) -> Self {
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Standard,
            RequestRecipient::Endpoint,
            REQ_CLEAR_FEATURE,
            0x0000, // ENDPOINT_HALT = 0
            endpoint_addr as u16,
            0x0000,
        )
    }

    // -------------------------------------------------------------------------
    // HID Class Request Constructors
    // -------------------------------------------------------------------------

    /// Builds a HID `SET_PROTOCOL` request (0 = Boot Protocol, 1 = Report Protocol).
    pub const fn hid_set_protocol(interface: u16, protocol: u16) -> Self {
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Class,
            RequestRecipient::Interface,
            HID_REQ_SET_PROTOCOL,
            protocol,
            interface,
            0x0000,
        )
    }

    /// Builds a HID `SET_IDLE` request to suppress periodic reporting when unchanged.
    pub const fn hid_set_idle(interface: u16, duration_4ms: u8, report_id: u8) -> Self {
        let w_value = ((duration_4ms as u16) << 8) | (report_id as u16);
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Class,
            RequestRecipient::Interface,
            HID_REQ_SET_IDLE,
            w_value,
            interface,
            0x0000,
        )
    }

    /// Builds a HID `GET_REPORT` request.
    pub const fn hid_get_report(interface: u16, report_type: u8, report_id: u8, length: u16) -> Self {
        let w_value = ((report_type as u16) << 8) | (report_id as u16);
        Self::new(
            RequestDirection::DeviceToHost,
            RequestType::Class,
            RequestRecipient::Interface,
            HID_REQ_GET_REPORT,
            w_value,
            interface,
            length,
        )
    }

    // -------------------------------------------------------------------------
    // Mass Storage Class Request Constructors
    // -------------------------------------------------------------------------

    /// Builds a Bulk-Only Mass Storage Reset class request.
    pub const fn msc_bot_reset(interface: u16) -> Self {
        Self::new(
            RequestDirection::HostToDevice,
            RequestType::Class,
            RequestRecipient::Interface,
            MSC_BOT_RESET,
            0x0000,
            interface,
            0x0000,
        )
    }

    /// Builds a Bulk-Only Mass Storage Get Max LUN class request.
    pub const fn msc_get_max_lun(interface: u16) -> Self {
        Self::new(
            RequestDirection::DeviceToHost,
            RequestType::Class,
            RequestRecipient::Interface,
            MSC_BOT_GET_MAX_LUN,
            0x0000,
            interface,
            0x0001,
        )
    }
}
