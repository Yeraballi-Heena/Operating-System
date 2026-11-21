//! Integrate OS — Central PCI Bus Manager & Enumerator
//!
//! Scans physical PCI buses, enumerates functions, discovers PCI-to-PCI bridges,
//! and provides fast hardware lookup by vendor/device ID or class.

use alloc::vec::Vec;
use crate::pci::address::PciAddress;
use crate::pci::device::PciDevice;
use crate::pci::types::PCI_SECONDARY_BUS;
use crate::sync::IrqSpinlock;
use crate::klog_info;

/// Central PCI / PCIe hardware manager and device repository.
pub struct PciManager {
    pub devices: Vec<PciDevice>,
    pub buses_scanned: u8,
}

impl PciManager {
    pub const fn new() -> Self {
        Self {
            devices: Vec::new(),
            buses_scanned: 0,
        }
    }

    /// Performs full systematic enumeration across physical PCI buses.
    pub fn scan_buses(&mut self) {
        self.devices.clear();

        // Enumerate primary root buses (0..8 covering standard chipsets & bridges)
        for bus in 0..8 {
            self.scan_bus(bus);
        }
        self.buses_scanned = 8;
    }

    /// Scans all 32 device slots on a specific PCI `bus`.
    pub fn scan_bus(&mut self, bus: u8) {
        for dev in 0..32 {
            let addr0 = PciAddress::new(bus, dev, 0);
            if let Some(dev0) = PciDevice::probe(addr0) {
                let is_multi = dev0.is_multi_function;
                let is_bridge = dev0.class_code == 0x06 && dev0.subclass == 0x04;

                self.devices.push(dev0);

                // If device is a PCI-to-PCI bridge, probe secondary bus
                if is_bridge {
                    let sec_bus = addr0.read_u8(PCI_SECONDARY_BUS);
                    if sec_bus > bus && sec_bus < 32 {
                        self.scan_bus(sec_bus);
                    }
                }

                // If multi-function device, probe remaining functions (1..7)
                if is_multi {
                    for func in 1..8 {
                        let addr = PciAddress::new(bus, dev, func);
                        if let Some(dev_fn) = PciDevice::probe(addr) {
                            self.devices.push(dev_fn);
                        }
                    }
                }
            }
        }
    }

    /// Finds a device by exact 16-bit Vendor ID and Device ID.
    pub fn find_device(&self, vendor_id: u16, device_id: u16) -> Option<&PciDevice> {
        self.devices
            .iter()
            .find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
    }

    /// Finds all devices matching a specific Base Class and Subclass.
    pub fn find_by_class(&self, class: u8, subclass: u8) -> Vec<&PciDevice> {
        self.devices
            .iter()
            .filter(|d| d.class_code == class && d.subclass == subclass)
            .collect()
    }

    /// Returns the total count of discovered PCI devices.
    pub fn count(&self) -> usize {
        self.devices.len()
    }

    /// Prints a formatted inventory of all discovered PCI devices to the kernel log.
    pub fn log_inventory(&self) {
        klog_info!("[pci] Enumerated {} physical PCI/PCIe functions:", self.devices.len());
        for dev in &self.devices {
            klog_info!(
                "[pci]   [{}] {:04X}:{:04X} | Class {:02X}:{:02X} ({} - {}) | IRQ {}",
                dev.addr.to_string(),
                dev.vendor_id,
                dev.device_id,
                dev.class_code,
                dev.subclass,
                dev.class_name(),
                dev.subclass_name(),
                dev.interrupt_line
            );
        }
    }
}

/// Global thread-safe PCI Manager instance.
pub static PCI_MANAGER: IrqSpinlock<PciManager> = IrqSpinlock::new(PciManager::new());
