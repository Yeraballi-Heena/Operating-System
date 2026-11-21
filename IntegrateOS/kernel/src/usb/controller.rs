//! Integrate OS — USB Host Controller Abstraction
//!
//! Provides the generic `UsbHostController` trait unifying UHCI, OHCI, EHCI, xHCI,
//! and virtual host controller implementations.

use crate::pci::PciAddress;
use super::setup::UsbSetupPacket;
use super::types::*;

/// Classification of USB host controller interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbControllerType {
    /// Intel Universal Host Controller Interface (USB 1.1)
    Uhci,
    /// Open Host Controller Interface (USB 1.1)
    Ohci,
    /// Enhanced Host Controller Interface (USB 2.0)
    Ehci,
    /// Extensible Host Controller Interface (USB 3.0+)
    Xhci,
    /// Virtual / Software Emulated Controller for verification & testing
    Virtual,
}

impl UsbControllerType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Uhci => "UHCI (USB 1.1)",
            Self::Ohci => "OHCI (USB 1.1)",
            Self::Ehci => "EHCI (USB 2.0)",
            Self::Xhci => "xHCI (USB 3.0+)",
            Self::Virtual => "Virtual USB Host Controller",
        }
    }
}

/// Generic interface implemented by all USB host controllers.
pub trait UsbHostController: Send + Sync {
    /// Returns the human-readable controller label.
    fn name(&self) -> &'static str;

    /// Returns the controller architecture type.
    fn controller_type(&self) -> UsbControllerType;

    /// Returns the PCI bus address of the host controller if physically present.
    fn pci_address(&self) -> Option<PciAddress>;

    /// Initializes controller registers, DMA rings, and enables interrupts/mastering.
    fn init(&mut self) -> Result<(), UsbError>;

    /// Polls controller status registers and processes completed transfer rings.
    fn poll(&mut self);

    /// Returns the number of physical or root downstream ports.
    fn port_count(&self) -> u8;

    /// Returns `true` if a device is electrically connected to the specified port.
    fn is_port_connected(&self, port: u8) -> bool;

    /// Issues a bus reset sequence on the specified port.
    fn reset_port(&mut self, port: u8) -> Result<(), UsbError>;

    /// Executes a synchronous USB Control Transfer on Endpoint 0.
    fn control_transfer(
        &mut self,
        addr: u8,
        setup: &UsbSetupPacket,
        data: Option<&mut [u8]>,
    ) -> Result<usize, UsbError>;

    /// Executes a synchronous USB Bulk Transfer (e.g. Mass Storage).
    fn bulk_transfer(
        &mut self,
        addr: u8,
        endpoint: u8,
        dir: RequestDirection,
        data: &mut [u8],
    ) -> Result<usize, UsbError>;

    /// Executes a synchronous USB Interrupt Transfer (e.g. HID report polling).
    fn interrupt_transfer(
        &mut self,
        addr: u8,
        endpoint: u8,
        dir: RequestDirection,
        data: &mut [u8],
    ) -> Result<usize, UsbError>;
}
