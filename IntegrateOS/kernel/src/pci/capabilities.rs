//! Integrate OS — PCI Capability Linked-List & MSI / MSI-X Parser
//!
//! Traverses the PCI configuration space capability linked list starting at offset 0x34.

use alloc::vec::Vec;
use crate::pci::address::PciAddress;
use crate::pci::types::{PCI_CAPABILITIES_PTR, PCI_STATUS, PCI_STATUS_CAPABILITIES_LIST};

pub const CAP_ID_PM: u8 = 0x01;
pub const CAP_ID_AGP: u8 = 0x02;
pub const CAP_ID_VPD: u8 = 0x03;
pub const CAP_ID_SLOT_ID: u8 = 0x04;
pub const CAP_ID_MSI: u8 = 0x05;
pub const CAP_ID_VENDOR: u8 = 0x09;
pub const CAP_ID_PCIE: u8 = 0x10;
pub const CAP_ID_MSIX: u8 = 0x11;
pub const CAP_ID_SATA: u8 = 0x12;

/// Detailed classification of a PCI capability.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapabilityType {
    PowerManagement,
    Msi {
        message_control: u16,
        is_64bit: bool,
        enabled: bool,
    },
    PciExpress {
        version: u8,
        device_type: u8,
    },
    MsiX {
        table_size: u16,
        enabled: bool,
    },
    VendorSpecific,
    Other(u8),
}

/// A parsed PCI Capability structure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PciCapability {
    pub id: u8,
    pub offset: u8,
    pub cap_type: CapabilityType,
}

/// Traverses the capability list for `addr` and parses all implemented capabilities.
pub fn parse_capabilities(addr: &PciAddress) -> Vec<PciCapability> {
    let mut caps = Vec::new();

    let status = addr.read_u16(PCI_STATUS);
    if (status & PCI_STATUS_CAPABILITIES_LIST) == 0 {
        return caps;
    }

    let mut ptr = addr.read_u8(PCI_CAPABILITIES_PTR) & 0xFC;
    let mut visited = [false; 256];

    while ptr >= 0x40 && !visited[ptr as usize] {
        visited[ptr as usize] = true;

        let cap_id = addr.read_u8(ptr);
        let next_ptr = addr.read_u8(ptr + 1) & 0xFC;

        let cap_type = match cap_id {
            CAP_ID_PM => CapabilityType::PowerManagement,
            CAP_ID_MSI => {
                let msg_ctrl = addr.read_u16(ptr + 2);
                let is_64bit = (msg_ctrl & (1 << 7)) != 0;
                let enabled = (msg_ctrl & (1 << 0)) != 0;
                CapabilityType::Msi {
                    message_control: msg_ctrl,
                    is_64bit,
                    enabled,
                }
            }
            CAP_ID_PCIE => {
                let pcie_flags = addr.read_u16(ptr + 2);
                let version = (pcie_flags & 0x0F) as u8;
                let device_type = ((pcie_flags >> 4) & 0x0F) as u8;
                CapabilityType::PciExpress {
                    version,
                    device_type,
                }
            }
            CAP_ID_MSIX => {
                let msg_ctrl = addr.read_u16(ptr + 2);
                let table_size = (msg_ctrl & 0x07FF) + 1;
                let enabled = (msg_ctrl & (1 << 15)) != 0;
                CapabilityType::MsiX {
                    table_size,
                    enabled,
                }
            }
            CAP_ID_VENDOR => CapabilityType::VendorSpecific,
            other => CapabilityType::Other(other),
        };

        caps.push(PciCapability {
            id: cap_id,
            offset: ptr,
            cap_type,
        });

        ptr = next_ptr;
    }

    caps
}
