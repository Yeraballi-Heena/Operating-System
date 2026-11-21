//! Concrete Driver Wrappers for Initial Subsystems
//!
//! Wraps Phase 1 and Phase 4 hardware drivers into unified `Driver` trait objects.

use crate::arch::x86_64::keyboard::KEYBOARD_QUEUE;
use crate::arch::x86_64::pit;
use crate::drivers::device::{
    AcpiDriver, DeviceCategory, DisplayDriver, Driver, FramebufferDriver, KeyboardDriver,
    RtcDriver, StorageDriver, TimerDriver,
};
use crate::drivers::traits::HardwareSupportStatus;
use crate::serial;
use crate::vga;

/// 8254 Programmable Interval Timer (PIT) Driver Wrapper.
pub struct PitTimerDevice {
    target_freq: u32,
}

impl PitTimerDevice {
    pub const fn new(target_freq: u32) -> Self {
        Self { target_freq }
    }
}

impl Driver for PitTimerDevice {
    fn name(&self) -> &str {
        "Intel 8254 PIT"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Timer
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        pit::init(self.target_freq);
        Ok(())
    }
}

impl TimerDriver for PitTimerDevice {
    fn current_ticks(&self) -> u64 {
        pit::get_ticks()
    }

    fn frequency_hz(&self) -> u32 {
        self.target_freq
    }
}

/// PS/2 Keyboard Driver Wrapper.
pub struct Ps2KeyboardDevice;

impl Driver for Ps2KeyboardDevice {
    fn name(&self) -> &str {
        "Standard PS/2 Keyboard"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Keyboard
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl KeyboardDriver for Ps2KeyboardDevice {
    fn read_char(&mut self) -> Option<u8> {
        KEYBOARD_QUEUE.lock().pop()
    }
}

/// 80x25 VGA Text Mode Display Driver Wrapper.
pub struct VgaDisplayDevice;

impl Driver for VgaDisplayDevice {
    fn name(&self) -> &str {
        "VGA Text Mode Console (80x25)"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Display
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        vga::init();
        Ok(())
    }
}

impl DisplayDriver for VgaDisplayDevice {
    fn write_char(&mut self, c: u8) {
        vga::write_byte(c);
    }

    fn clear_screen(&mut self) {
        vga::init();
    }
}

/// Bochs / VirtualBox VBE Linear Framebuffer Device Wrapper.
pub struct BgaFramebufferDevice;

impl Driver for BgaFramebufferDevice {
    fn name(&self) -> &str {
        "Bochs / VirtualBox VBE Framebuffer (1024x768x32)"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Display
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::graphics::bga::is_bga_present() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        crate::graphics::init(1024, 768, 32).map(|_| ())
    }
}

impl DisplayDriver for BgaFramebufferDevice {
    fn write_char(&mut self, c: u8) {
        crate::graphics::with_graphics(|canvas, console| {
            console.write_char(canvas, c as char);
            canvas.present_rect(console.bounds);
        });
    }

    fn clear_screen(&mut self) {
        crate::graphics::with_graphics(|canvas, console| {
            console.clear(canvas);
            canvas.present_rect(console.bounds);
        });
    }
}

impl FramebufferDriver for BgaFramebufferDevice {
    fn resolution(&self) -> (u32, u32) {
        crate::graphics::GRAPHICS
            .lock()
            .as_ref()
            .map(|g| (g.mode.width, g.mode.height))
            .unwrap_or((1024, 768))
    }

    fn pitch(&self) -> u32 {
        crate::graphics::GRAPHICS
            .lock()
            .as_ref()
            .map(|g| g.mode.pitch)
            .unwrap_or(1024 * 4)
    }

    fn bpp(&self) -> u8 {
        32
    }

    fn framebuffer_address(&self) -> u64 {
        crate::graphics::GRAPHICS
            .lock()
            .as_ref()
            .map(|g| g.mode.phys_addr)
            .unwrap_or(0xE000_0000)
    }

    fn present(&mut self) {
        crate::graphics::with_canvas(|c| c.present());
    }
}

/// 16550 UART Serial Communication Driver Wrapper.
pub struct Com1SerialDevice;

impl Driver for Com1SerialDevice {
    fn name(&self) -> &str {
        "16550 UART Serial (COM1)"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Serial
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        serial::init();
        Ok(())
    }
}

/// IDE / ATA Block Storage Controller Device Driver.
pub struct StorageFoundationDevice;

impl Driver for StorageFoundationDevice {
    fn name(&self) -> &str {
        "IDE / ATA Block Storage Controller"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Storage
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::storage::is_storage_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl StorageDriver for StorageFoundationDevice {
    fn block_size(&self) -> usize {
        512
    }

    fn total_blocks(&self) -> u64 {
        crate::storage::total_blocks()
    }

    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        crate::storage::read_block(block, buf)
    }

    fn write_block(&mut self, block: u64, buf: &[u8]) -> Result<(), &'static str> {
        crate::storage::write_block(block, buf)
    }
}

/// Generic Network Interface Foundation (Phase 23 placeholder).
pub struct NetworkFoundationDevice;

impl Driver for NetworkFoundationDevice {
    fn name(&self) -> &str {
        "Generic Ethernet Interface"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Network
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::net::is_network_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Generic PCI Bus Foundation (Phase 24 placeholder).
pub struct PciFoundationDevice;

impl Driver for PciFoundationDevice {
    fn name(&self) -> &str {
        "PCI Bus Controller"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Pci
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::pci::is_pci_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Generic ACPI Power Foundation (Phase 29).
pub struct AcpiFoundationDevice;

impl Driver for AcpiFoundationDevice {
    fn name(&self) -> &str {
        "ACPI Power Management System"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Acpi
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::acpi::is_acpi_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl AcpiDriver for AcpiFoundationDevice {
    fn reboot(&mut self) -> Result<(), &'static str> {
        crate::acpi::reboot();
    }

    fn power_off(&mut self) -> Result<(), &'static str> {
        crate::acpi::power_off();
    }
}

/// Generic USB Subsystem Foundation (Phase 25 placeholder).
pub struct UsbFoundationDevice;

impl Driver for UsbFoundationDevice {
    fn name(&self) -> &str {
        "USB Host Controller (xHCI/EHCI)"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Usb
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::usb::is_usb_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Generic GPU Subsystem Foundation (Phase 26).
pub struct GpuFoundationDevice;

impl Driver for GpuFoundationDevice {
    fn name(&self) -> &str {
        "PCI Graphics Adapter / GPU"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Display
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::gpu::is_gpu_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Generic Audio Subsystem Foundation (Phase 27).
pub struct AudioFoundationDevice;

impl Driver for AudioFoundationDevice {
    fn name(&self) -> &str {
        "PCI Audio Controller / Sound Card"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Audio
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::audio::is_audio_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Motorola MC146818 CMOS Real-Time Clock Device Driver (Phase 28).
pub struct CmosRtcDevice;

impl Driver for CmosRtcDevice {
    fn name(&self) -> &str {
        "Motorola MC146818 CMOS RTC"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Rtc
    }

    fn status(&self) -> HardwareSupportStatus {
        if crate::time::is_time_initialized() {
            HardwareSupportStatus::Supported
        } else {
            HardwareSupportStatus::Foundation
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl RtcDriver for CmosRtcDevice {
    fn read_timestamp_seconds(&self) -> u64 {
        crate::time::timestamp()
    }
}
