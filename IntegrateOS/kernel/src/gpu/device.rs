//! Integrate OS — GPU Hardware Device Representation
//!
//! Encapsulates physical PCI Display Controller information, BAR memory mappings,
//! and VRAM aperture layout.

use alloc::format;
use alloc::string::String;
use crate::pci::address::PciAddress;
use crate::pci::bar::Bar;
use crate::pci::device::PciDevice;
use super::types::*;

/// Representation of an identified PCI Display Controller.
#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub addr: PciAddress,
    pub vendor: GpuVendor,
    pub vendor_id: u16,
    pub device_id: u16,
    pub subsystem_id: u16,
    pub model_name: String,
    pub vram_size: u64,
    pub bars: [Option<Bar>; 6],
    pub lfb_bar_index: usize,
    pub mmio_bar_index: Option<usize>,
    pub io_bar_index: Option<usize>,
}

impl GpuDevice {
    /// Inspects a PCI device and populates a GPU device descriptor.
    pub fn from_pci(pci_dev: &PciDevice) -> Self {
        let vendor = GpuVendor::from_vendor_id(pci_dev.vendor_id);
        let model_name = match (pci_dev.vendor_id, pci_dev.device_id) {
            (0x80EE, 0xBEEF) => String::from("VirtualBox Graphics Adapter (VBE/BGA)"),
            (0x1234, 0x1111) => String::from("QEMU Standard VGA / Bochs Display"),
            (0x15AD, 0x0405) => String::from("VMware SVGA II Graphics Adapter"),
            (0x8086, dev) => format!("Intel HD/UHD Integrated Graphics (Device 0x{:04X})", dev),
            (0x1002, dev) => format!("AMD Radeon Display Adapter (Device 0x{:04X})", dev),
            (0x10DE, dev) => format!("NVIDIA GeForce Graphics (Device 0x{:04X})", dev),
            _ => format!("Standard VGA Display Controller (0x{:04X}:0x{:04X})", pci_dev.vendor_id, pci_dev.device_id),
        };

        // Determine VRAM size and LFB BAR from probed BARs
        let mut vram_size = 16 * 1024 * 1024; // Default 16 MiB fallback
        let mut lfb_idx = 0;
        let mut mmio_idx = None;
        let mut io_idx = None;

        for (idx, bar_opt) in pci_dev.bars.iter().enumerate() {
            if let Some(bar) = bar_opt {
                match bar {
                    Bar::Memory { size, .. } => {
                        if *size > vram_size {
                            vram_size = *size;
                            lfb_idx = idx;
                        } else if mmio_idx.is_none() {
                            mmio_idx = Some(idx);
                        }
                    }
                    Bar::Io { .. } => {
                        if io_idx.is_none() {
                            io_idx = Some(idx);
                        }
                    }
                }
            }
        }

        Self {
            addr: pci_dev.addr,
            vendor,
            vendor_id: pci_dev.vendor_id,
            device_id: pci_dev.device_id,
            subsystem_id: pci_dev.subsystem_id,
            model_name,
            vram_size,
            bars: pci_dev.bars,
            lfb_bar_index: lfb_idx,
            mmio_bar_index: mmio_idx,
            io_bar_index: io_idx,
        }
    }

    /// Returns the physical Linear Framebuffer base address.
    pub fn lfb_phys_address(&self) -> u64 {
        if let Some(Some(bar)) = self.bars.get(self.lfb_bar_index) {
            bar.mmio_address().unwrap_or(crate::graphics::bga::DEFAULT_BGA_LFB_PHYS_ADDR)
        } else {
            crate::graphics::bga::DEFAULT_BGA_LFB_PHYS_ADDR
        }
    }

    /// Formats a hardware inventory summary string.
    pub fn format_description(&self) -> String {
        format!(
            "[{}] {:04X}:{:04X} '{}' (VRAM: {} MiB, LFB: 0x{:08X})",
            self.addr.to_string(),
            self.vendor_id,
            self.device_id,
            self.model_name,
            self.vram_size / (1024 * 1024),
            self.lfb_phys_address()
        )
    }
}
