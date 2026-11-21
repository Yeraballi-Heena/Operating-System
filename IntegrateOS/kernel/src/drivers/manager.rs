//! Central Device Manager & Registry
//!
//! Manages registration, enumeration, lookup, and status tracking for all hardware
//! drivers in the operating system.

use crate::drivers::device::{DeviceCategory, Driver};
use crate::drivers::traits::HardwareSupportStatus;
use crate::sync::IrqSpinlock;
use alloc::boxed::Box;
use alloc::vec::Vec;

pub struct DeviceRegistry {
    drivers: Vec<Box<dyn Driver>>,
}

impl DeviceRegistry {
    pub const fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    /// Registers a new driver instance into the central registry.
    pub fn register(&mut self, mut driver: Box<dyn Driver>) -> Result<(), &'static str> {
        driver.init()?;
        self.drivers.push(driver);
        Ok(())
    }

    /// Returns the number of registered devices.
    pub fn count(&self) -> usize {
        self.drivers.len()
    }

    /// Returns the driver at index `idx`.
    pub fn get(&self, idx: usize) -> Option<&Box<dyn Driver>> {
        self.drivers.get(idx)
    }

    /// Finds all drivers belonging to a specific category.
    pub fn find_by_category(&self, category: DeviceCategory) -> Vec<&Box<dyn Driver>> {
        self.drivers
            .iter()
            .filter(|d| d.category() == category)
            .collect()
    }

    /// Counts drivers by their operational status.
    pub fn count_by_status(&self, status: HardwareSupportStatus) -> usize {
        self.drivers
            .iter()
            .filter(|d| d.status() == status)
            .count()
    }
}

pub static DEVICE_MANAGER: IrqSpinlock<DeviceRegistry> = IrqSpinlock::new(DeviceRegistry::new());

/// Initializes the central device manager and registers the baseline hardware devices.
pub fn init() -> Result<(), &'static str> {
    use crate::drivers::wrappers::*;

    let mut manager = DEVICE_MANAGER.lock();

    // Register active operational hardware
    manager.register(Box::new(PitTimerDevice::new(100)))?;
    manager.register(Box::new(Ps2KeyboardDevice))?;
    manager.register(Box::new(VgaDisplayDevice))?;
    manager.register(Box::new(BgaFramebufferDevice))?;
    manager.register(Box::new(Com1SerialDevice))?;

    // Register architectural foundation placeholders
    manager.register(Box::new(StorageFoundationDevice))?;
    manager.register(Box::new(NetworkFoundationDevice))?;
    manager.register(Box::new(PciFoundationDevice))?;
    manager.register(Box::new(AcpiFoundationDevice))?;
    manager.register(Box::new(UsbFoundationDevice))?;
    manager.register(Box::new(GpuFoundationDevice))?;
    manager.register(Box::new(AudioFoundationDevice))?;
    manager.register(Box::new(CmosRtcDevice))?;

    Ok(())
}
