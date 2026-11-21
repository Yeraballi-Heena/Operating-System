//! Integrate OS — UHCI (Universal Host Controller Interface USB 1.1)
//!
//! Provides Intel UHCI port I/O register definitions and controller abstraction.

use crate::pci::PciAddress;
use super::super::controller::{UsbControllerType, UsbHostController};
use super::super::setup::UsbSetupPacket;
use super::super::types::*;

pub const UHCI_REG_USBCMD: u16 = 0x00;
pub const UHCI_REG_USBSTS: u16 = 0x02;
pub const UHCI_REG_USBINTR: u16 = 0x04;
pub const UHCI_REG_FRNUM: u16 = 0x06;
pub const UHCI_REG_FRBASEADD: u16 = 0x08;
pub const UHCI_REG_SOFMOD: u16 = 0x0C;
pub const UHCI_REG_PORTSC1: u16 = 0x10;
pub const UHCI_REG_PORTSC2: u16 = 0x12;

pub const UHCI_CMD_RUN_STOP: u16 = 1 << 0;
pub const UHCI_CMD_HCRESET: u16 = 1 << 1;
pub const UHCI_CMD_GRESET: u16 = 1 << 2;
pub const UHCI_CMD_MAXP: u16 = 1 << 7;

pub const UHCI_STS_USBINT: u16 = 1 << 0;
pub const UHCI_STS_ERROR: u16 = 1 << 1;
pub const UHCI_STS_HALTED: u16 = 1 << 5;

pub const UHCI_PORTSC_CCS: u16 = 1 << 0; // Current Connect Status
pub const UHCI_PORTSC_CSC: u16 = 1 << 1; // Connect Status Change
pub const UHCI_PORTSC_PE: u16 = 1 << 2;  // Port Enabled
pub const UHCI_PORTSC_PR: u16 = 1 << 9;  // Port Reset

/// Physical or emulated UHCI Host Controller instance.
pub struct UhciController {
    pci_addr: PciAddress,
    io_base: u16,
    initialized: bool,
}

impl UhciController {
    pub fn new(pci_addr: PciAddress, io_base: u16) -> Self {
        Self {
            pci_addr,
            io_base,
            initialized: false,
        }
    }

    pub fn io_base(&self) -> u16 {
        self.io_base
    }
}

impl UsbHostController for UhciController {
    fn name(&self) -> &'static str {
        "UHCI USB 1.1 Host Controller"
    }

    fn controller_type(&self) -> UsbControllerType {
        UsbControllerType::Uhci
    }

    fn pci_address(&self) -> Option<PciAddress> {
        Some(self.pci_addr)
    }

    fn init(&mut self) -> Result<(), UsbError> {
        if self.io_base == 0 {
            return Err(UsbError::ControllerNotFound);
        }
        self.initialized = true;
        Ok(())
    }

    fn poll(&mut self) {}

    fn port_count(&self) -> u8 {
        2
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
