//! Unified Driver Framework & Device Traits
//!
//! Provides modular hardware driver traits:
//! - `Driver`: Base trait for device lifecycle (probe, init, reset, shutdown)
//! - `TimerDriver`: Hardware tick and interval generation
//! - `InputDriver`: Keyboard and mouse event reporting
//! - `DisplayDriver`: Text console and linear framebuffer rendering
//! - `StorageDriver`: Block-level storage access (read/write sectors)
//! - `NetworkDriver`: Ethernet packet transmission and reception
//! - `PciDriver`: PCI bus device enumeration and configuration
//! - `RtcDriver`: Real-Time Clock date and timestamp querying
//! - `AcpiDriver`: Power state transitions and system shutdown
//! - `UsbDriver`: USB host controller and device management

use crate::drivers::traits::HardwareSupportStatus;

/// Device Category classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCategory {
    Display,
    Keyboard,
    Mouse,
    Timer,
    Storage,
    Pci,
    Network,
    Rtc,
    Acpi,
    Usb,
    Serial,
    Audio,
}

impl DeviceCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Timer => "Timer",
            Self::Storage => "Storage",
            Self::Pci => "PCI",
            Self::Network => "Network",
            Self::Rtc => "RTC",
            Self::Acpi => "ACPI",
            Self::Usb => "USB",
            Self::Serial => "Serial UART",
            Self::Audio => "Audio",
        }
    }
}

/// Base trait that every hardware device driver must implement.
pub trait Driver: Send + Sync {
    /// Human-readable device driver name (e.g. "Intel 8254 PIT").
    fn name(&self) -> &str;

    /// Device classification category.
    fn category(&self) -> DeviceCategory;

    /// Current hardware operational status.
    fn status(&self) -> HardwareSupportStatus;

    /// Initializes the underlying hardware controller.
    fn init(&mut self) -> Result<(), &'static str>;

    /// Optional device shutdown or reset handler.
    fn shutdown(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Trait for system timers and monotonic clocks.
pub trait TimerDriver: Driver {
    fn current_ticks(&self) -> u64;
    fn frequency_hz(&self) -> u32;
}

/// Trait for keyboard input hardware.
pub trait KeyboardDriver: Driver {
    fn read_char(&mut self) -> Option<u8>;
}

/// Trait for mouse and pointing input hardware.
pub trait MouseDriver: Driver {
    fn position(&self) -> (i32, i32);
    fn buttons(&self) -> u8;
}

/// Trait for display devices (text or graphical).
pub trait DisplayDriver: Driver {
    fn write_char(&mut self, c: u8);
    fn clear_screen(&mut self);
}

/// Trait for high-resolution linear framebuffer display devices.
pub trait FramebufferDriver: DisplayDriver {
    fn resolution(&self) -> (u32, u32);
    fn pitch(&self) -> u32;
    fn bpp(&self) -> u8;
    fn framebuffer_address(&self) -> u64;
    fn present(&mut self);
}

/// Trait for block storage controllers.
pub trait StorageDriver: Driver {
    fn block_size(&self) -> usize;
    fn total_blocks(&self) -> u64;
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<(), &'static str>;
}

/// Trait for network interface cards.
pub trait NetworkDriver: Driver {
    fn mac_address(&self) -> [u8; 6];
    fn send_packet(&mut self, packet: &[u8]) -> Result<(), &'static str>;
    fn receive_packet(&mut self, buf: &mut [u8]) -> Result<usize, &'static str>;
}

/// Trait for Real-Time Clock hardware.
pub trait RtcDriver: Driver {
    fn read_timestamp_seconds(&self) -> u64;
}

/// Trait for ACPI power management controllers.
pub trait AcpiDriver: Driver {
    fn reboot(&mut self) -> Result<(), &'static str>;
    fn power_off(&mut self) -> Result<(), &'static str>;
}
