//! Integrate OS — USB Types & Specification Constants
//!
//! Standard definitions according to the USB 2.0 and USB 3.x specifications:
//! - Speeds (Low, Full, High, SuperSpeed)
//! - Transfer types (Control, Isochronous, Bulk, Interrupt)
//! - Request directions, types, and recipients
//! - Standard request codes (GET_DESCRIPTOR, SET_ADDRESS, SET_CONFIGURATION, etc.)
//! - Standard descriptor type codes (Device, Config, String, Interface, Endpoint, HID, Hub)
//! - Device class identifiers (HID, Mass Storage, Hub, Audio, CDC, etc.)
//! - USB error conditions

/// Operating speed of a USB device or host port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// USB 1.0/1.1 Low Speed (1.5 Mbps) - Keyboards, Mice
    LowSpeed,
    /// USB 1.1 Full Speed (12 Mbps) - Audio, early Flash drives
    FullSpeed,
    /// USB 2.0 High Speed (480 Mbps) - High-throughput peripherals
    HighSpeed,
    /// USB 3.0 SuperSpeed (5 Gbps) - External SSDs, Displays
    SuperSpeed,
    /// USB 3.1+ SuperSpeedPlus (10 Gbps+)
    SuperSpeedPlus,
}

impl UsbSpeed {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::LowSpeed => "Low-Speed (1.5 Mbps)",
            Self::FullSpeed => "Full-Speed (12 Mbps)",
            Self::HighSpeed => "High-Speed (480 Mbps)",
            Self::SuperSpeed => "SuperSpeed (5 Gbps)",
            Self::SuperSpeedPlus => "SuperSpeedPlus (10 Gbps)",
        }
    }
}

/// Fundamental USB transfer pipe type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbTransferType {
    /// Bidirectional message transfer for device enumeration and control requests.
    Control = 0,
    /// Periodic, guaranteed latency and bandwidth transfers (streaming audio/video).
    Isochronous = 1,
    /// Non-periodic, high-volume guaranteed error-free transfer (mass storage, printers).
    Bulk = 2,
    /// Periodic, low-frequency polling transfers (keyboards, mice).
    Interrupt = 3,
}

impl UsbTransferType {
    pub const fn from_bm_attributes(attr: u8) -> Self {
        match attr & 0x03 {
            0 => Self::Control,
            1 => Self::Isochronous,
            2 => Self::Bulk,
            _ => Self::Interrupt,
        }
    }
}

/// Data direction of a USB packet or endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestDirection {
    /// Host transmits to Device (OUT, 0)
    HostToDevice = 0,
    /// Device transmits to Host (IN, 1)
    DeviceToHost = 1,
}

impl RequestDirection {
    pub const fn from_u8(val: u8) -> Self {
        if (val & 0x80) != 0 {
            Self::DeviceToHost
        } else {
            Self::HostToDevice
        }
    }

    pub const fn to_bit(&self) -> u8 {
        match self {
            Self::HostToDevice => 0x00,
            Self::DeviceToHost => 0x80,
        }
    }
}

/// Standard request classifications in `bmRequestType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    /// USB Specification Standard Request
    Standard = 0,
    /// USB Device Class Specific Request (e.g. HID, MSC)
    Class = 1,
    /// Hardware Vendor Specific Request
    Vendor = 2,
    /// Reserved
    Reserved = 3,
}

impl RequestType {
    pub const fn from_u8(val: u8) -> Self {
        match (val >> 5) & 0x03 {
            0 => Self::Standard,
            1 => Self::Class,
            2 => Self::Vendor,
            _ => Self::Reserved,
        }
    }

    pub const fn to_bits(&self) -> u8 {
        match self {
            Self::Standard => 0x00,
            Self::Class => 0x20,
            Self::Vendor => 0x40,
            Self::Reserved => 0x60,
        }
    }
}

/// Request recipient in `bmRequestType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRecipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}

impl RequestRecipient {
    pub const fn from_u8(val: u8) -> Self {
        match val & 0x1F {
            0 => Self::Device,
            1 => Self::Interface,
            2 => Self::Endpoint,
            _ => Self::Other,
        }
    }

    pub const fn to_bits(&self) -> u8 {
        match self {
            Self::Device => 0x00,
            Self::Interface => 0x01,
            Self::Endpoint => 0x02,
            Self::Other => 0x03,
        }
    }
}

// -----------------------------------------------------------------------------
// Standard USB Requests (Table 9-4)
// -----------------------------------------------------------------------------
pub const REQ_GET_STATUS: u8 = 0x00;
pub const REQ_CLEAR_FEATURE: u8 = 0x01;
pub const REQ_SET_FEATURE: u8 = 0x03;
pub const REQ_SET_ADDRESS: u8 = 0x05;
pub const REQ_GET_DESCRIPTOR: u8 = 0x06;
pub const REQ_SET_DESCRIPTOR: u8 = 0x07;
pub const REQ_GET_CONFIGURATION: u8 = 0x08;
pub const REQ_SET_CONFIGURATION: u8 = 0x09;
pub const REQ_GET_INTERFACE: u8 = 0x0A;
pub const REQ_SET_INTERFACE: u8 = 0x0B;
pub const REQ_SYNCH_FRAME: u8 = 0x0C;

// -----------------------------------------------------------------------------
// Standard Descriptor Types (Table 9-5)
// -----------------------------------------------------------------------------
pub const DESC_DEVICE: u8 = 0x01;
pub const DESC_CONFIGURATION: u8 = 0x02;
pub const DESC_STRING: u8 = 0x03;
pub const DESC_INTERFACE: u8 = 0x04;
pub const DESC_ENDPOINT: u8 = 0x05;
pub const DESC_DEVICE_QUALIFIER: u8 = 0x06;
pub const DESC_OTHER_SPEED_CONFIGURATION: u8 = 0x07;
pub const DESC_INTERFACE_POWER: u8 = 0x08;
pub const DESC_OTG: u8 = 0x09;
pub const DESC_DEBUG: u8 = 0x0A;
pub const DESC_INTERFACE_ASSOCIATION: u8 = 0x0B;
pub const DESC_BOS: u8 = 0x0F;
pub const DESC_DEVICE_CAPABILITY: u8 = 0x10;
pub const DESC_HID: u8 = 0x21;
pub const DESC_HID_REPORT: u8 = 0x22;
pub const DESC_HID_PHYSICAL: u8 = 0x23;
pub const DESC_HUB: u8 = 0x29;
pub const DESC_SUPERSPEED_HUB: u8 = 0x2A;

// -----------------------------------------------------------------------------
// Standard USB Device Class Codes
// -----------------------------------------------------------------------------
pub const CLASS_PER_INTERFACE: u8 = 0x00;
pub const CLASS_AUDIO: u8 = 0x01;
pub const CLASS_CDC_CONTROL: u8 = 0x02;
pub const CLASS_HID: u8 = 0x03;
pub const CLASS_PHYSICAL: u8 = 0x05;
pub const CLASS_IMAGE: u8 = 0x06;
pub const CLASS_PRINTER: u8 = 0x07;
pub const CLASS_MASS_STORAGE: u8 = 0x08;
pub const CLASS_HUB: u8 = 0x09;
pub const CLASS_CDC_DATA: u8 = 0x0A;
pub const CLASS_SMART_CARD: u8 = 0x0B;
pub const CLASS_VIDEO: u8 = 0x0E;
pub const CLASS_WIRELESS: u8 = 0xE0;
pub const CLASS_MISCELLANEOUS: u8 = 0xEF;
pub const CLASS_APPLICATION: u8 = 0xFE;
pub const CLASS_VENDOR: u8 = 0xFF;

/// Decodes human-readable class name string.
pub fn get_class_name(class_code: u8) -> &'static str {
    match class_code {
        CLASS_PER_INTERFACE => "Defined at Interface Level",
        CLASS_AUDIO => "Audio Device",
        CLASS_CDC_CONTROL => "Communications and CDC Control",
        CLASS_HID => "Human Interface Device (HID)",
        CLASS_PHYSICAL => "Physical Device",
        CLASS_IMAGE => "Still Imaging Device",
        CLASS_PRINTER => "Printer Device",
        CLASS_MASS_STORAGE => "Mass Storage Device",
        CLASS_HUB => "USB Hub",
        CLASS_CDC_DATA => "CDC Data Interface",
        CLASS_SMART_CARD => "Smart Card Reader",
        CLASS_VIDEO => "Video Device",
        CLASS_WIRELESS => "Wireless Controller",
        CLASS_MISCELLANEOUS => "Miscellaneous Device",
        CLASS_APPLICATION => "Application Specific",
        CLASS_VENDOR => "Vendor Specific Device",
        _ => "Unknown Class",
    }
}

// -----------------------------------------------------------------------------
// HID Class Requests & Constants
// -----------------------------------------------------------------------------
pub const HID_REQ_GET_REPORT: u8 = 0x01;
pub const HID_REQ_GET_IDLE: u8 = 0x02;
pub const HID_REQ_GET_PROTOCOL: u8 = 0x03;
pub const HID_REQ_SET_REPORT: u8 = 0x09;
pub const HID_REQ_SET_IDLE: u8 = 0x0A;
pub const HID_REQ_SET_PROTOCOL: u8 = 0x0B;

pub const HID_PROTOCOL_BOOT: u16 = 0x00;
pub const HID_PROTOCOL_REPORT: u16 = 0x01;

pub const HID_BOOT_SUBCLASS: u8 = 0x01;
pub const HID_KEYBOARD_PROTOCOL: u8 = 0x01;
pub const HID_MOUSE_PROTOCOL: u8 = 0x02;

// -----------------------------------------------------------------------------
// Mass Storage Class Constants
// -----------------------------------------------------------------------------
pub const MSC_SUBCLASS_RBC: u8 = 0x01;
pub const MSC_SUBCLASS_ATAPI: u8 = 0x02;
pub const MSC_SUBCLASS_QIC: u8 = 0x03;
pub const MSC_SUBCLASS_UFI: u8 = 0x04;
pub const MSC_SUBCLASS_SFF8070I: u8 = 0x05;
pub const MSC_SUBCLASS_SCSI: u8 = 0x06;

pub const MSC_PROTOCOL_CBI_INTERRUPT: u8 = 0x00;
pub const MSC_PROTOCOL_CBI_NO_INTERRUPT: u8 = 0x01;
pub const MSC_PROTOCOL_BOT: u8 = 0x50; // Bulk-Only Transport

pub const MSC_BOT_RESET: u8 = 0xFF;
pub const MSC_BOT_GET_MAX_LUN: u8 = 0xFE;

// -----------------------------------------------------------------------------
// USB Error Definitions
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbError {
    ControllerNotFound,
    DeviceNotFound,
    EndpointNotFound,
    Timeout,
    Stall,
    Babble,
    CrcError,
    BufferTooSmall,
    InvalidDescriptor,
    UnsupportedProtocol,
    IoError,
    PhaseError,
}

impl UsbError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ControllerNotFound => "USB Host Controller not found",
            Self::DeviceNotFound => "USB Device not found",
            Self::EndpointNotFound => "USB Endpoint not found",
            Self::Timeout => "USB transfer timed out",
            Self::Stall => "USB pipe stalled (STALL handshake received)",
            Self::Babble => "USB babble detected on bus",
            Self::CrcError => "USB CRC or bitstuffing error",
            Self::BufferTooSmall => "Transfer buffer too small",
            Self::InvalidDescriptor => "Malformed or invalid descriptor received",
            Self::UnsupportedProtocol => "Unsupported USB protocol or class",
            Self::IoError => "Low-level hardware I/O error",
            Self::PhaseError => "SCSI/BOT Phase error",
        }
    }
}
