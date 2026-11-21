//! Bochs / VirtualBox VBE Graphics Adapter (BGA) Driver & PCI LFB Probe
//!
//! Provides hardware mode switching (e.g. 1024x768x32bpp) via standard BGA I/O ports
//! (`0x1CE` / `0x1CF`) and discovers linear framebuffer base via PCI configuration space.

use crate::arch::x86_64::io::{inw, outw, pci_read_u32};

pub const VBE_DISPI_IOPORT_INDEX: u16 = 0x01CE;
pub const VBE_DISPI_IOPORT_DATA: u16 = 0x01CF;

pub const VBE_DISPI_INDEX_ID: u16 = 0x0;
pub const VBE_DISPI_INDEX_XRES: u16 = 0x1;
pub const VBE_DISPI_INDEX_YRES: u16 = 0x2;
pub const VBE_DISPI_INDEX_BPP: u16 = 0x3;
pub const VBE_DISPI_INDEX_ENABLE: u16 = 0x4;
pub const VBE_DISPI_INDEX_BANK: u16 = 0x5;
pub const VBE_DISPI_INDEX_VIRT_WIDTH: u16 = 0x6;
pub const VBE_DISPI_INDEX_VIRT_HEIGHT: u16 = 0x7;
pub const VBE_DISPI_INDEX_X_OFFSET: u16 = 0x8;
pub const VBE_DISPI_INDEX_Y_OFFSET: u16 = 0x9;
pub const VBE_DISPI_INDEX_VIDEO_MEMORY_64K: u16 = 0xA;

pub const VBE_DISPI_DISABLED: u16 = 0x00;
pub const VBE_DISPI_ENABLED: u16 = 0x01;
pub const VBE_DISPI_GETCAPS: u16 = 0x02;
pub const VBE_DISPI_8BIT_DAC: u16 = 0x04;
pub const VBE_DISPI_LFB_ENABLED: u16 = 0x40;
pub const VBE_DISPI_NOCLEARMEM: u16 = 0x80;

pub const VBE_DISPI_ID0: u16 = 0xB0C0;
pub const VBE_DISPI_ID1: u16 = 0xB0C1;
pub const VBE_DISPI_ID2: u16 = 0xB0C2;
pub const VBE_DISPI_ID3: u16 = 0xB0C3;
pub const VBE_DISPI_ID4: u16 = 0xB0C4;
pub const VBE_DISPI_ID5: u16 = 0xB0C5;

/// Default physical base address of VirtualBox / QEMU / Bochs Linear Framebuffer.
pub const DEFAULT_BGA_LFB_PHYS_ADDR: u64 = 0xE000_0000;

#[derive(Debug, Clone, Copy)]
pub struct BgaModeInfo {
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub pitch: u32,
    pub phys_addr: u64,
}

/// Reads a 16-bit register from the Bochs VBE Graphics Adapter.
#[inline]
pub fn bga_read(index: u16) -> u16 {
    // SAFETY: Accessing standard BGA indexing ports 0x1CE and 0x1CF.
    unsafe {
        outw(VBE_DISPI_IOPORT_INDEX, index);
        inw(VBE_DISPI_IOPORT_DATA)
    }
}

/// Writes a 16-bit register to the Bochs VBE Graphics Adapter.
#[inline]
pub fn bga_write(index: u16, val: u16) {
    // SAFETY: Accessing standard BGA indexing ports 0x1CE and 0x1CF.
    unsafe {
        outw(VBE_DISPI_IOPORT_INDEX, index);
        outw(VBE_DISPI_IOPORT_DATA, val);
    }
}

/// Detects if Bochs / VirtualBox VBE (BGA) hardware is present.
pub fn is_bga_present() -> bool {
    bga_write(VBE_DISPI_INDEX_ID, VBE_DISPI_ID5);
    let id = bga_read(VBE_DISPI_INDEX_ID);
    (VBE_DISPI_ID0..=VBE_DISPI_ID5).contains(&id)
}

/// Scans PCI configuration space to discover the Display Controller (Class 0x03)
/// and reads Base Address Register 0 (BAR0) for the linear framebuffer physical address.
pub fn detect_pci_framebuffer() -> Option<u64> {
    for bus in 0..=4 {
        for dev in 0..32 {
            for func in 0..8 {
                // SAFETY: Reading standard PCI configuration space header.
                let vendor_device = unsafe { pci_read_u32(bus, dev, func, 0x00) };
                let vendor_id = (vendor_device & 0xFFFF) as u16;
                if vendor_id == 0xFFFF {
                    if func == 0 {
                        break; // Skip non-existent multi-function devices
                    }
                    continue;
                }

                // SAFETY: Reading PCI class code at offset 0x08.
                let class_rev = unsafe { pci_read_u32(bus, dev, func, 0x08) };
                let class_code = ((class_rev >> 24) & 0xFF) as u8;

                if class_code == 0x03 {
                    // Display Controller found! Read BAR0 (offset 0x10).
                    // SAFETY: Reading BAR0 register of identified display adapter.
                    let bar0 = unsafe { pci_read_u32(bus, dev, func, 0x10) };
                    let lfb_phys = (bar0 & 0xFFFF_FFF0) as u64;
                    if lfb_phys != 0 {
                        return Some(lfb_phys);
                    }
                }
            }
        }
    }
    None
}

/// Discovers the active physical address of the linear framebuffer.
pub fn get_framebuffer_phys_addr() -> u64 {
    if let Some(pci_lfb) = detect_pci_framebuffer() {
        pci_lfb
    } else {
        DEFAULT_BGA_LFB_PHYS_ADDR
    }
}

/// Programs the BGA controller to the specified resolution and color depth.
pub fn set_video_mode(width: u32, height: u32, bpp: u8) -> Result<BgaModeInfo, &'static str> {
    if !is_bga_present() {
        return Err("BGA graphics adapter not detected");
    }

    // 1. Temporarily disable BGA
    bga_write(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);

    // 2. Set resolution and color depth
    bga_write(VBE_DISPI_INDEX_XRES, width as u16);
    bga_write(VBE_DISPI_INDEX_YRES, height as u16);
    bga_write(VBE_DISPI_INDEX_BPP, bpp as u16);

    // 3. Re-enable with Linear Framebuffer (LFB) mode while preserving VRAM
    bga_write(
        VBE_DISPI_INDEX_ENABLE,
        VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED | VBE_DISPI_NOCLEARMEM,
    );

    let bytes_per_pixel = (bpp as u32) / 8;
    let pitch = width * bytes_per_pixel;
    let phys_addr = get_framebuffer_phys_addr();

    Ok(BgaModeInfo {
        width,
        height,
        bpp,
        pitch,
        phys_addr,
    })
}

/// Disables the BGA display controller, returning the video adapter to VGA text mode.
pub fn disable_bga() {
    bga_write(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);
}
