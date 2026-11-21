//! Integrate OS — xHCI (Extensible Host Controller Interface USB 3.0+)
//!
//! Provides xHCI hardware register mapping, capability decoding, and controller management.

use crate::pci::PciAddress;
use super::super::controller::{UsbControllerType, UsbHostController};
use super::super::setup::UsbSetupPacket;
use super::super::types::*;

// Capability Register Offsets
pub const XHCI_CAP_CAPLENGTH: usize = 0x00;
pub const XHCI_CAP_HCIVERSION: usize = 0x02;
pub const XHCI_CAP_HCSPARAMS1: usize = 0x04;
pub const XHCI_CAP_HCSPARAMS2: usize = 0x08;
pub const XHCI_CAP_HCSPARAMS3: usize = 0x0C;
pub const XHCI_CAP_HCCPARAMS1: usize = 0x10;
pub const XHCI_CAP_DBOFF: usize = 0x14;
pub const XHCI_CAP_RTSOFF: usize = 0x18;

// Operational Register Offsets (relative to op_base = base + caplength)
pub const XHCI_OP_USBCMD: usize = 0x00;
pub const XHCI_OP_USBSTS: usize = 0x04;
pub const XHCI_OP_PAGESIZE: usize = 0x08;
pub const XHCI_OP_DNCTRL: usize = 0x14;
pub const XHCI_OP_CRCR: usize = 0x18;
pub const XHCI_OP_DCBAAP: usize = 0x30;
pub const XHCI_OP_CONFIG: usize = 0x38;
pub const XHCI_OP_PORTS_BASE: usize = 0x400;

// USBCMD Flags
pub const USBCMD_RUN_STOP: u32 = 1 << 0;
pub const USBCMD_RESET: u32 = 1 << 1;
pub const USBCMD_INTE: u32 = 1 << 2;

// USBSTS Flags
pub const USBSTS_HALTED: u32 = 1 << 0;
pub const USBSTS_HSE: u32 = 1 << 2;
pub const USBSTS_EINT: u32 = 1 << 3;
pub const USBSTS_PCD: u32 = 1 << 4;
pub const USBSTS_CNR: u32 = 1 << 11;

// PORTSC Flags
pub const PORTSC_CCS: u32 = 1 << 0; // Current Connect Status
pub const PORTSC_PED: u32 = 1 << 1; // Port Enabled/Disabled
pub const PORTSC_PR: u32 = 1 << 4;  // Port Reset
pub const PORTSC_PLS_MASK: u32 = 0x0F << 5; // Port Link State
pub const PORTSC_PP: u32 = 1 << 9;  // Port Power
pub const PORTSC_SPEED_MASK: u32 = 0x0F << 10;

/// Physical or emulated xHCI Host Controller instance.
pub struct XhciController {
    pci_addr: PciAddress,
    mmio_base: u64,
    cap_length: u8,
    hci_version: u16,
    max_slots: u8,
    max_ports: u8,
    initialized: bool,
}

impl XhciController {
    pub fn new(pci_addr: PciAddress, mmio_base: u64) -> Self {
        Self {
            pci_addr,
            mmio_base,
            cap_length: 0x20, // typical xHCI caplength
            hci_version: 0x0100,
            max_slots: 8,
            max_ports: 4,
            initialized: false,
        }
    }

    pub fn mmio_base(&self) -> u64 {
        self.mmio_base
    }

    pub fn cap_length(&self) -> u8 {
        self.cap_length
    }

    pub fn hci_version(&self) -> u16 {
        self.hci_version
    }

    pub fn max_slots(&self) -> u8 {
        self.max_slots
    }

    pub fn max_ports(&self) -> u8 {
        self.max_ports
    }
}

impl UsbHostController for XhciController {
    fn name(&self) -> &'static str {
        "xHCI USB 3.0 Host Controller"
    }

    fn controller_type(&self) -> UsbControllerType {
        UsbControllerType::Xhci
    }

    fn pci_address(&self) -> Option<PciAddress> {
        Some(self.pci_addr)
    }

    fn init(&mut self) -> Result<(), UsbError> {
        // Safe check for MMIO base sanity
        if self.mmio_base == 0 {
            return Err(UsbError::ControllerNotFound);
        }
        self.initialized = true;
        Ok(())
    }

    fn poll(&mut self) {
        // Poll event rings when physical controller is active
    }

    fn port_count(&self) -> u8 {
        self.max_ports
    }

    fn is_port_connected(&self, _port: u8) -> bool {
        false
    }

    fn reset_port(&mut self, _port: u8) -> Result<(), UsbError> {
        Ok(())
    }

    fn control_transfer(
        &mut self,
        _addr: u8,
        _setup: &UsbSetupPacket,
        _data: Option<&mut [u8]>,
    ) -> Result<usize, UsbError> {
        Err(UsbError::DeviceNotFound)
    }

    fn bulk_transfer(
        &mut self,
        _addr: u8,
        _endpoint: u8,
        _dir: RequestDirection,
        _data: &mut [u8],
    ) -> Result<usize, UsbError> {
        Err(UsbError::DeviceNotFound)
    }

    fn interrupt_transfer(
        &mut self,
        _addr: u8,
        _endpoint: u8,
        _dir: RequestDirection,
        _data: &mut [u8],
    ) -> Result<usize, UsbError> {
        Err(UsbError::DeviceNotFound)
    }
}
