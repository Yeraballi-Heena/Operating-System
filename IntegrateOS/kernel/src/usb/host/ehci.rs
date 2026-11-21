//! Integrate OS — EHCI (Enhanced Host Controller Interface USB 2.0)
//!
//! Provides EHCI hardware register mapping and operational state management.

use crate::pci::PciAddress;
use super::super::controller::{UsbControllerType, UsbHostController};
use super::super::setup::UsbSetupPacket;
use super::super::types::*;

pub const EHCI_CAP_CAPLENGTH: usize = 0x00;
pub const EHCI_CAP_HCIVERSION: usize = 0x02;
pub const EHCI_CAP_HCSPARAMS: usize = 0x04;
pub const EHCI_CAP_HCCPARAMS: usize = 0x08;

pub const EHCI_OP_USBCMD: usize = 0x00;
pub const EHCI_OP_USBSTS: usize = 0x04;
pub const EHCI_OP_USBINTR: usize = 0x08;
pub const EHCI_OP_FRINDEX: usize = 0x0C;
pub const EHCI_OP_PERIODICLISTBASE: usize = 0x14;
pub const EHCI_OP_ASYNCLISTADDR: usize = 0x18;
pub const EHCI_OP_CONFIGFLAG: usize = 0x40;
pub const EHCI_OP_PORTSC_BASE: usize = 0x44;

pub const EHCI_CMD_RUN_STOP: u32 = 1 << 0;
pub const EHCI_CMD_RESET: u32 = 1 << 1;
pub const EHCI_CMD_PERIODIC_ENABLE: u32 = 1 << 4;
pub const EHCI_CMD_ASYNC_ENABLE: u32 = 1 << 5;

pub const EHCI_STS_INT: u32 = 1 << 0;
pub const EHCI_STS_ERR_INT: u32 = 1 << 1;
pub const EHCI_STS_PORT_CHANGE: u32 = 1 << 2;
pub const EHCI_STS_HALTED: u32 = 1 << 12;

pub const EHCI_PORTSC_CCS: u32 = 1 << 0;  // Current Connect Status
pub const EHCI_PORTSC_CSC: u32 = 1 << 1;  // Connect Status Change
pub const EHCI_PORTSC_PED: u32 = 1 << 2;  // Port Enabled
pub const EHCI_PORTSC_PR: u32 = 1 << 8;   // Port Reset
pub const EHCI_PORTSC_PP: u32 = 1 << 12;  // Port Power

/// Physical or emulated EHCI Host Controller instance.
pub struct EhciController {
    pci_addr: PciAddress,
    mmio_base: u64,
    port_count: u8,
    initialized: bool,
}

impl EhciController {
    pub fn new(pci_addr: PciAddress, mmio_base: u64) -> Self {
        Self {
            pci_addr,
            mmio_base,
            port_count: 4,
            initialized: false,
        }
    }

    pub fn mmio_base(&self) -> u64 {
        self.mmio_base
    }
}

impl UsbHostController for EhciController {
    fn name(&self) -> &'static str {
        "EHCI USB 2.0 Host Controller"
    }

    fn controller_type(&self) -> UsbControllerType {
        UsbControllerType::Ehci
    }

    fn pci_address(&self) -> Option<PciAddress> {
        Some(self.pci_addr)
    }

    fn init(&mut self) -> Result<(), UsbError> {
        if self.mmio_base == 0 {
            return Err(UsbError::ControllerNotFound);
        }
        self.initialized = true;
        Ok(())
    }

    fn poll(&mut self) {}

    fn port_count(&self) -> u8 {
        self.port_count
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
