//! Integrate OS — Enumerated USB Device Representation
//!
//! Encapsulates physical or virtual USB device descriptors, active configuration,
//! discovered endpoints, and device class classification.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use super::descriptors::*;
use super::types::*;

/// Representation of an active, enumerated USB peripheral.
#[derive(Debug, Clone)]
pub struct UsbDevice {
    /// Assigned USB Bus Address (1..127).
    pub address: u8,
    /// Negotiated link speed.
    pub speed: UsbSpeed,
    /// Index of managing host controller.
    pub controller_id: usize,
    /// Root or downstream port number.
    pub port: u8,
    /// Device Descriptor.
    pub descriptor: UsbDeviceDescriptor,
    /// Active configuration descriptor.
    pub active_config: UsbConfigurationDescriptor,
    /// Discovered interfaces with endpoints.
    pub interfaces: Vec<ParsedInterface>,
    /// Active configuration value (typically 1).
    pub current_configuration: u8,
    /// Decoded manufacturer string.
    pub manufacturer: Option<String>,
    /// Decoded product description string.
    pub product: Option<String>,
    /// Decoded serial number string.
    pub serial: Option<String>,
}

impl UsbDevice {
    pub fn vendor_id(&self) -> u16 {
        self.descriptor.id_vendor
    }

    pub fn product_id(&self) -> u16 {
        self.descriptor.id_product
    }

    /// Returns `true` if any interface implements the Human Interface Device class (0x03).
    pub fn is_hid(&self) -> bool {
        self.interfaces.iter().any(|iface| iface.interface.b_interface_class == CLASS_HID)
    }

    /// Returns `true` if any interface is a HID Boot Keyboard.
    pub fn is_hid_keyboard(&self) -> bool {
        self.interfaces.iter().any(|iface| {
            iface.interface.b_interface_class == CLASS_HID
                && iface.interface.b_interface_subclass == HID_BOOT_SUBCLASS
                && iface.interface.b_interface_protocol == HID_KEYBOARD_PROTOCOL
        })
    }

    /// Returns `true` if any interface is a HID Boot Mouse.
    pub fn is_hid_mouse(&self) -> bool {
        self.interfaces.iter().any(|iface| {
            iface.interface.b_interface_class == CLASS_HID
                && iface.interface.b_interface_subclass == HID_BOOT_SUBCLASS
                && iface.interface.b_interface_protocol == HID_MOUSE_PROTOCOL
        })
    }

    /// Returns `true` if any interface is a Mass Storage Bulk-Only SCSI device.
    pub fn is_mass_storage(&self) -> bool {
        self.interfaces.iter().any(|iface| {
            iface.interface.b_interface_class == CLASS_MASS_STORAGE
                && iface.interface.b_interface_subclass == MSC_SUBCLASS_SCSI
                && iface.interface.b_interface_protocol == MSC_PROTOCOL_BOT
        })
    }

    /// Returns `true` if the device is a USB Hub.
    pub fn is_hub(&self) -> bool {
        self.descriptor.b_device_class == CLASS_HUB
            || self.interfaces.iter().any(|iface| iface.interface.b_interface_class == CLASS_HUB)
    }

    /// Locates an endpoint matching transfer type and direction across all active interfaces.
    pub fn find_endpoint(
        &self,
        transfer_type: UsbTransferType,
        dir: RequestDirection,
    ) -> Option<UsbEndpointDescriptor> {
        for iface in &self.interfaces {
            for ep in &iface.endpoints {
                if ep.transfer_type() == transfer_type && ep.direction() == dir {
                    return Some(*ep);
                }
            }
        }
        None
    }

    /// Formats a concise hardware description string for console logging.
    pub fn format_description(&self) -> String {
        let prod = self.product.as_deref().unwrap_or("Unknown Product");
        let mfg = self.manufacturer.as_deref().unwrap_or("Generic");
        format!(
            "[USB Addr {:02}] {:04X}:{:04X} '{}' by '{}' ({})",
            self.address,
            self.vendor_id(),
            self.product_id(),
            prod,
            mfg,
            self.speed.as_str()
        )
    }
}
