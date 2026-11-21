//! Integrate OS — GPU Subsystem Manager & Driver Dispatcher
//!
//! Orchestrates PCI display adapter discovery, driver selection, display mode switching,
//! and global 2D graphics acceleration routing.

use alloc::boxed::Box;
use crate::pci::PCI_MANAGER;
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};
use super::device::GpuDevice;
use super::drivers::{QemuStdVgaDriver, SoftwareGpuDriver, VboxGpuDriver, VmwareSvgaDriver};
use super::traits::GpuDriver;
use super::types::*;

pub struct GpuManager {
    detected_device: Option<GpuDevice>,
    driver: Box<dyn GpuDriver>,
}

impl GpuManager {
    pub fn new() -> Self {
        Self {
            detected_device: None,
            driver: Box::new(SoftwareGpuDriver::new()),
        }
    }

    /// Discovers physical PCI display controllers and instantiates the optimal driver.
    pub fn discover_and_init(&mut self) -> Result<(), GpuError> {
        let display_devices = {
            let mgr = PCI_MANAGER.lock();
            // Class 0x03 is Display Controller
            let mut list = mgr.find_by_class(0x03, 0x00); // VGA-Compatible
            if list.is_empty() {
                list = mgr.find_by_class(0x03, 0x80); // Other Display
            }
            list.into_iter().cloned().collect::<alloc::vec::Vec<_>>()
        };

        if let Some(pci_dev) = display_devices.first() {
            let gpu_dev = GpuDevice::from_pci(pci_dev);
            klog_info!("[gpu] Discovered physical PCI GPU: {}", gpu_dev.format_description());

            let driver: Box<dyn GpuDriver> = match (gpu_dev.vendor_id, gpu_dev.device_id) {
                (0x80EE, 0xBEEF) => {
                    klog_info!("[gpu] Binding VirtualBox VBE/BGA Graphics Driver");
                    Box::new(VboxGpuDriver::new(gpu_dev.clone()))
                }
                (0x1234, 0x1111) => {
                    klog_info!("[gpu] Binding QEMU Standard VGA Driver");
                    Box::new(QemuStdVgaDriver::new(gpu_dev.clone()))
                }
                (0x15AD, 0x0405) => {
                    klog_info!("[gpu] Binding VMware SVGA II Driver");
                    Box::new(VmwareSvgaDriver::new(gpu_dev.clone()))
                }
                _ => {
                    klog_info!("[gpu] Binding generic VBE-compatible PCI Display Driver");
                    Box::new(VboxGpuDriver::new(gpu_dev.clone()))
                }
            };

            self.detected_device = Some(gpu_dev);
            self.driver = driver;
        } else {
            klog_info!("[gpu] No PCI display adapter detected; falling back to software rasterizer");
            self.driver = Box::new(SoftwareGpuDriver::new());
        }

        self.driver.init()?;
        let mode = self.driver.current_mode();
        let caps = self.driver.capabilities();
        klog_ok!(
            "[gpu] Active GPU Driver: '{}' (Mode: {}x{}x{}bpp, VRAM: {} MiB, HW 2D: {})",
            self.driver.name(),
            mode.width,
            mode.height,
            mode.bpp,
            caps.vram_bytes / (1024 * 1024),
            if caps.hw_2d_accel { "Yes" } else { "No" }
        );

        Ok(())
    }

    pub fn driver(&self) -> &dyn GpuDriver {
        &*self.driver
    }

    pub fn driver_mut(&mut self) -> &mut dyn GpuDriver {
        &mut *self.driver
    }

    pub fn detected_device(&self) -> Option<&GpuDevice> {
        self.detected_device.as_ref()
    }
}

pub static GPU_MANAGER: IrqSpinlock<Option<GpuManager>> = IrqSpinlock::new(None);

/// Executes a closure with mutable access to the active GPU driver.
pub fn with_gpu_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut dyn GpuDriver) -> R,
{
    let mut lock = GPU_MANAGER.lock();
    lock.as_mut().map(|mgr| f(mgr.driver_mut()))
}
