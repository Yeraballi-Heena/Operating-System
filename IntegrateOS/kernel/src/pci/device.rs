//! Integrate OS — PCI Device Representation & Hardware Control
//!
//! Encapsulates full configuration space state, BAR ranges, interrupt routing,
//! and hardware capability inspection for a probed PCI function.

use alloc::vec::Vec;
use crate::pci::address::PciAddress;
use crate::pci::bar::Bar;
use crate::pci::capabilities::{parse_capabilities, PciCapability};
use crate::pci::classes::{get_class_name, get_prog_if_name, get_subclass_name};
use crate::pci::types::*;

/// Complete representation of an enumerated PCI function.
#[derive(Clone, Debug)]
pub struct PciDevice {
    pub addr: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class_code: u8,
    pub header_type: u8,
    pub is_multi_function: bool,
    pub bars: [Option<Bar>; 6],
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub capabilities: Vec<PciCapability>,
}

impl PciDevice {
    /// Probes a physical PCI address and populates a device record if present.
    pub fn probe(addr: PciAddress) -> Option<Self> {
        let vendor_id = addr.read_u16(PCI_VENDOR_ID);
        // 0xFFFF indicates an unpopulated or unresponsive PCI slot
        if vendor_id == 0xFFFF || vendor_id == 0x0000 {
            return None;
        }

        let device_id = addr.read_u16(PCI_DEVICE_ID);
        let command = addr.read_u16(PCI_COMMAND);
        let status = addr.read_u16(PCI_STATUS);
        let revision_id = addr.read_u8(PCI_REVISION_ID);
        let prog_if = addr.read_u8(PCI_PROG_IF);
        let subclass = addr.read_u8(PCI_SUBCLASS);
        let class_code = addr.read_u8(PCI_CLASS);
        let header_raw = addr.read_u8(PCI_HEADER_TYPE);

        let header_type = header_raw & 0x7F;
        let is_multi_function = (header_raw & PCI_HEADER_TYPE_MULTIFUNCTION_MASK) != 0;

        // Probe Base Address Registers (BARs) for Type 0x00 General Devices
        let mut bars = [None, None, None, None, None, None];
        let mut sub_vendor = 0;
        let mut sub_id = 0;

        if header_type == PCI_HEADER_TYPE_DEVICE {
            let mut bar_idx = 0;
            while bar_idx < 6 {
                let (bar_opt, is_64bit_upper) = Bar::probe(&addr, bar_idx);
                bars[bar_idx as usize] = bar_opt;
                if is_64bit_upper {
                    bar_idx += 2; // Upper 32-bits consumed by current BAR
                } else {
                    bar_idx += 1;
                }
            }

            sub_vendor = addr.read_u16(PCI_SUBSYSTEM_VENDOR_ID);
            sub_id = addr.read_u16(PCI_SUBSYSTEM_ID);
        }

        let interrupt_line = addr.read_u8(PCI_INTERRUPT_LINE);
        let interrupt_pin = addr.read_u8(PCI_INTERRUPT_PIN);
        let capabilities = parse_capabilities(&addr);

        Some(Self {
            addr,
            vendor_id,
            device_id,
            command,
            status,
            revision_id,
            prog_if,
            subclass,
            class_code,
            header_type,
            is_multi_function,
            bars,
            interrupt_line,
            interrupt_pin,
            subsystem_vendor_id: sub_vendor,
            subsystem_id: sub_id,
            capabilities,
        })
    }

    /// Enables Bus Mastering (DMA) on this PCI device.
    pub fn enable_bus_mastering(&self) {
        let cmd = self.addr.read_u16(PCI_COMMAND);
        self.addr.write_u16(PCI_COMMAND, cmd | PCI_COMMAND_BUS_MASTER);
    }

    /// Enables Memory Space decoding for MMIO BARs.
    pub fn enable_memory_space(&self) {
        let cmd = self.addr.read_u16(PCI_COMMAND);
        self.addr.write_u16(PCI_COMMAND, cmd | PCI_COMMAND_MEMORY_SPACE);
    }

    /// Enables I/O Space decoding for Port I/O BARs.
    pub fn enable_io_space(&self) {
        let cmd = self.addr.read_u16(PCI_COMMAND);
        self.addr.write_u16(PCI_COMMAND, cmd | PCI_COMMAND_IO_SPACE);
    }

    /// Returns the descriptive Base Class name.
    pub fn class_name(&self) -> &'static str {
        get_class_name(self.class_code)
    }

    /// Returns the descriptive Subclass name.
    pub fn subclass_name(&self) -> &'static str {
        get_subclass_name(self.class_code, self.subclass)
    }

    /// Returns the descriptive Programming Interface name.
    pub fn prog_if_name(&self) -> &'static str {
        get_prog_if_name(self.class_code, self.subclass, self.prog_if)
    }
}
