//! Integrate OS — PCI Base Address Register (BAR) Decoding
//!
//! Provides dynamic discovery, sizing, and address mapping for Memory-Mapped I/O (MMIO)
//! and legacy Port I/O device ranges.

use crate::pci::address::PciAddress;
use crate::pci::types::PCI_BAR0;

/// Base Address Register (BAR) descriptor.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bar {
    /// Memory-Mapped I/O region (32-bit or 64-bit).
    Memory {
        address: u64,
        size: u64,
        prefetchable: bool,
        is_64bit: bool,
    },
    /// Legacy x86 Port I/O range.
    Io {
        port: u16,
        size: u32,
    },
}

impl Bar {
    /// Probes and decodes BAR `bar_idx` (0..5) on `addr`.
    /// Returns `(Option<Bar>, is_64bit_continuation)`.
    pub fn probe(addr: &PciAddress, bar_idx: u8) -> (Option<Bar>, bool) {
        if bar_idx >= 6 {
            return (None, false);
        }

        let offset = PCI_BAR0 + bar_idx * 4;
        let original_val = addr.read_u32(offset);

        // 1. Probe BAR size: write all 1s and read back hardware mask
        addr.write_u32(offset, 0xFFFF_FFFF);
        let size_mask = addr.read_u32(offset);
        addr.write_u32(offset, original_val); // Restore original configuration

        // If all 0s or unlatched, BAR is unimplemented
        if size_mask == 0 || size_mask == 0xFFFF_FFFF {
            return (None, false);
        }

        // 2. Check BAR Type (bit 0: 1 = I/O, 0 = Memory)
        if (original_val & 0x01) == 1 {
            // Port I/O BAR
            let port = (original_val & 0xFFFF_FFFC) as u16;
            let size = (!(size_mask & 0xFFFF_FFFC)).wrapping_add(1);
            (Some(Bar::Io { port, size }), false)
        } else {
            // Memory Space BAR
            let bar_type = (original_val >> 1) & 0x03;
            let prefetchable = ((original_val >> 3) & 0x01) == 1;
            let is_64bit = bar_type == 0x02;

            if is_64bit && bar_idx < 5 {
                // 64-bit Memory BAR spans this BAR and the next BAR
                let offset_high = offset + 4;
                let original_high = addr.read_u32(offset_high);

                addr.write_u32(offset_high, 0xFFFF_FFFF);
                let size_mask_high = addr.read_u32(offset_high);
                addr.write_u32(offset_high, original_high);

                let full_address =
                    ((original_high as u64) << 32) | ((original_val & 0xFFFF_FFF0) as u64);
                let full_mask =
                    ((size_mask_high as u64) << 32) | ((size_mask & 0xFFFF_FFF0) as u64);
                let size = (!full_mask).wrapping_add(1);

                (
                    Some(Bar::Memory {
                        address: full_address,
                        size,
                        prefetchable,
                        is_64bit: true,
                    }),
                    true,
                )
            } else {
                // 32-bit Memory BAR
                let address = (original_val & 0xFFFF_FFF0) as u64;
                let size = ((!(size_mask & 0xFFFF_FFF0)).wrapping_add(1)) as u64;

                (
                    Some(Bar::Memory {
                        address,
                        size,
                        prefetchable,
                        is_64bit: false,
                    }),
                    false,
                )
            }
        }
    }

    /// Returns the Memory-Mapped I/O base address if this is a Memory BAR.
    pub fn mmio_address(&self) -> Option<u64> {
        match self {
            Bar::Memory { address, .. } => Some(*address),
            _ => None,
        }
    }

    /// Returns the Port I/O port address if this is an I/O BAR.
    pub fn io_port(&self) -> Option<u16> {
        match self {
            Bar::Io { port, .. } => Some(*port),
            _ => None,
        }
    }
}
