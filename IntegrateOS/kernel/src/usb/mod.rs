//! Integrate OS — USB Subsystem Architecture
//!
//! Provides host controller discovery (UHCI, OHCI, EHCI, xHCI, Virtual), standard
//! USB pipe control transfers, descriptor parsing, automatic peripheral enumeration,
//! and class drivers for HID Boot Keyboards, Mice, and Mass Storage devices.

pub mod bus;
pub mod controller;
pub mod descriptors;
pub mod device;
pub mod hid;
pub mod host;
pub mod manager;
pub mod msc;
pub mod setup;
pub mod tests;
pub mod types;

pub use bus::UsbBus;
pub use controller::{UsbControllerType, UsbHostController};
pub use descriptors::*;
pub use device::UsbDevice;
pub use manager::{with_controller_mut, UsbManager, USB_MANAGER};
pub use setup::UsbSetupPacket;
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static USB_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the USB subsystem has completed initialization and enumeration.
pub fn is_usb_initialized() -> bool {
    USB_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the USB subsystem, probes physical PCI and virtual host controllers,
/// and executes bus enumeration across all connected ports.
pub fn init() {
    let (ctrl_count, dev_count) = {
        let mut mgr = USB_MANAGER.lock();
        mgr.discover_controllers();
        mgr.enumerate_all();
        (mgr.devices().len(), mgr.device_count())
    };

    USB_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[usb] USB Subsystem initialized ({} devices enumerated)", dev_count);
    let _ = ctrl_count;
}
