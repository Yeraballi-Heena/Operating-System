//! Integrate OS — GPU Types, Modes & Capability Specifications
//!
//! Provides fundamental data types for display modes, pixel formats, GPU capabilities,
//! hardware vendors, and graphics errors.

/// Hardware pixel format representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 32-bit BGRA (Blue, Green, Red, Alpha - standard x86 framebuffer default)
    Bgra8888,
    /// 32-bit RGBA (Red, Green, Blue, Alpha)
    Rgba8888,
    /// 24-bit BGR (Blue, Green, Red)
    Bgr888,
    /// 16-bit RGB 5:6:5 High Color
    Rgb565,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(&self) -> u8 {
        match self {
            Self::Bgra8888 | Self::Rgba8888 => 4,
            Self::Bgr888 => 3,
            Self::Rgb565 => 2,
        }
    }

    pub const fn bpp(&self) -> u8 {
        self.bytes_per_pixel() * 8
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Bgra8888 => "BGRA 32-bit",
            Self::Rgba8888 => "RGBA 32-bit",
            Self::Bgr888 => "BGR 24-bit",
            Self::Rgb565 => "RGB 16-bit (5:6:5)",
        }
    }
}

/// A specific resolution, color depth, and refresh rate supported by the display adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub refresh_rate_hz: u32,
    pub format: PixelFormat,
}

impl DisplayMode {
    pub const MODE_640X480: Self = Self {
        width: 640,
        height: 480,
        bpp: 32,
        refresh_rate_hz: 60,
        format: PixelFormat::Bgra8888,
    };

    pub const MODE_800X600: Self = Self {
        width: 800,
        height: 600,
        bpp: 32,
        refresh_rate_hz: 60,
        format: PixelFormat::Bgra8888,
    };

    pub const MODE_1024X768: Self = Self {
        width: 1024,
        height: 768,
        bpp: 32,
        refresh_rate_hz: 60,
        format: PixelFormat::Bgra8888,
    };

    pub const MODE_1280X720: Self = Self {
        width: 1280,
        height: 720,
        bpp: 32,
        refresh_rate_hz: 60,
        format: PixelFormat::Bgra8888,
    };

    pub const MODE_1920X1080: Self = Self {
        width: 1920,
        height: 1080,
        bpp: 32,
        refresh_rate_hz: 60,
        format: PixelFormat::Bgra8888,
    };

    pub const fn new(width: u32, height: u32, bpp: u8, refresh_rate_hz: u32) -> Self {
        let format = match bpp {
            16 => PixelFormat::Rgb565,
            24 => PixelFormat::Bgr888,
            _ => PixelFormat::Bgra8888,
        };
        Self {
            width,
            height,
            bpp,
            refresh_rate_hz,
            format,
        }
    }

    /// Computes row pitch in bytes.
    pub const fn pitch(&self) -> u32 {
        self.width * (self.bpp as u32 / 8)
    }

    /// Computes total framebuffer frame size in bytes.
    pub const fn framebuffer_size(&self) -> usize {
        (self.pitch() * self.height) as usize
    }

    /// Returns the simplified aspect ratio (e.g. (4, 3) or (16, 9)).
    pub fn aspect_ratio(&self) -> (u32, u32) {
        let gcd = fn_gcd(self.width, self.height);
        (self.width / gcd, self.height / gcd)
    }
}

const fn fn_gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Known hardware GPU vendors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    VirtualBox,
    Qemu,
    Vmware,
    Unknown(u16),
}

impl GpuVendor {
    pub const fn from_vendor_id(vendor_id: u16) -> Self {
        match vendor_id {
            0x8086 => Self::Intel,
            0x1002 => Self::Amd,
            0x10DE => Self::Nvidia,
            0x80EE => Self::VirtualBox,
            0x1234 => Self::Qemu,
            0x15AD => Self::Vmware,
            other => Self::Unknown(other),
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Intel => "Intel Corporation",
            Self::Amd => "Advanced Micro Devices (AMD)",
            Self::Nvidia => "NVIDIA Corporation",
            Self::VirtualBox => "Oracle VirtualBox Graphics",
            Self::Qemu => "QEMU / Bochs Virtual VGA",
            Self::Vmware => "VMware SVGA II",
            Self::Unknown(_) => "Generic Display Adapter",
        }
    }
}

/// Hardware features supported by the GPU driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuCapabilities {
    pub hw_2d_accel: bool,
    pub hw_cursor: bool,
    pub page_flipping: bool,
    pub vsync: bool,
    pub hw_scaling: bool,
    pub command_queue: bool,
    pub vram_bytes: u64,
}

impl GpuCapabilities {
    pub const fn software_baseline() -> Self {
        Self {
            hw_2d_accel: false,
            hw_cursor: false,
            page_flipping: true,
            vsync: false,
            hw_scaling: false,
            command_queue: true,
            vram_bytes: 16 * 1024 * 1024, // 16 MiB
        }
    }

    pub const fn vbox_accelerated(vram_bytes: u64) -> Self {
        Self {
            hw_2d_accel: true,
            hw_cursor: true,
            page_flipping: true,
            vsync: true,
            hw_scaling: false,
            command_queue: true,
            vram_bytes,
        }
    }
}

/// Physical and virtual mapping details of the linear framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferInfo {
    pub phys_addr: u64,
    pub virt_addr: u64,
    pub size_bytes: usize,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pub format: PixelFormat,
}

/// GPU subsystem error classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuError {
    DeviceNotFound,
    ModeNotSupported,
    AllocationFailed,
    InvalidSurface,
    CommandQueueFull,
    HardwareError,
    Timeout,
    UnsupportedOperation,
}

impl GpuError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "Display Controller not found on PCI bus",
            Self::ModeNotSupported => "Requested display mode is not supported by hardware",
            Self::AllocationFailed => "Out of video or host memory for GPU surface",
            Self::InvalidSurface => "Invalid or unallocated Surface ID",
            Self::CommandQueueFull => "GPU hardware command queue ring buffer is full",
            Self::HardwareError => "Underlying GPU hardware or MMIO error",
            Self::Timeout => "GPU command or VSync wait timed out",
            Self::UnsupportedOperation => "Requested GPU operation is unsupported on this device",
        }
    }
}
