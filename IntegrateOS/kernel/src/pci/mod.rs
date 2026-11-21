//! Integrate OS — PCI / PCIe Subsystem Architecture
//!
//! Provides PCI 2.3 and PCI Express hardware bus enumeration, configuration space
//! mechanism #1 access, BAR decoding, capability discovery, and driver binding.

pub mod address;
pub mod bar;
pub mod capabilities;
pub mod classes;
pub mod device;
pub mod manager;
pub mod tests;
pub mod types;

pub use address::PciAddress;
pub use bar::Bar;
pub use capabilities::{parse_capabilities, CapabilityType, PciCapability};
pub use classes::{get_class_name, get_prog_if_name, get_subclass_name};
pub use device::PciDevice;
pub use manager::{PciManager, PCI_MANAGER};
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static PCI_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the PCI subsystem has completed physical bus enumeration.
pub fn is_pci_initialized() -> bool {
    PCI_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the PCI / PCIe subsystem and performs hardware bus discovery.
pub fn init() {
    let count = {
        let mut mgr = PCI_MANAGER.lock();
        mgr.scan_buses();
        mgr.log_inventory();
        mgr.count()
    };

    PCI_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[pci] PCI Subsystem initialized ({} devices discovered)", count);
}
