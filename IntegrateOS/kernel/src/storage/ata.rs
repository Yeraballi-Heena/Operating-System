//! IDE / ATA PIO Block Storage Driver
//!
//! Provides direct hardware register I/O for ATA/ATAPI controllers in PIO mode:
//! - Drive detection and `IDENTIFY DEVICE` (0xEC) interrogation
//! - 28-bit LBA sector reads (command 0x20) and writes (command 0x30)
//! - Status polling with calibrated timeouts and cache flush (command 0xE7)

use alloc::string::String;
use crate::arch::x86_64::io::{inb, inw, outb, outw};

// --- ATA I/O Port Base Addresses ---
pub const PRIMARY_IO_BASE: u16 = 0x1F0;
pub const PRIMARY_CTRL_BASE: u16 = 0x3F6;

pub const SECONDARY_IO_BASE: u16 = 0x170;
pub const SECONDARY_CTRL_BASE: u16 = 0x376;

// --- ATA Register Offsets ---
pub const ATA_REG_DATA: u16 = 0;
pub const ATA_REG_ERROR_FEATURES: u16 = 1;
pub const ATA_REG_SECTOR_COUNT: u16 = 2;
pub const ATA_REG_LBA_LOW: u16 = 3;
pub const ATA_REG_LBA_MID: u16 = 4;
pub const ATA_REG_LBA_HIGH: u16 = 5;
pub const ATA_REG_DRIVE_SELECT: u16 = 6;
pub const ATA_REG_STATUS_COMMAND: u16 = 7;

// --- ATA Status Register Bits ---
pub const ATA_STATUS_ERR: u8 = 1 << 0; // Error
pub const ATA_STATUS_DRQ: u8 = 1 << 3; // Data Request ready
pub const ATA_STATUS_DF: u8 = 1 << 5;  // Device Fault
pub const ATA_STATUS_RDY: u8 = 1 << 6; // Device Ready
pub const ATA_STATUS_BSY: u8 = 1 << 7; // Busy

// --- ATA Commands ---
pub const ATA_CMD_READ_SECTORS: u8 = 0x20;
pub const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
pub const ATA_CMD_CACHE_FLUSH: u8 = 0xE7;
pub const ATA_CMD_IDENTIFY: u8 = 0xEC;

/// ATA Bus Channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaBus {
    Primary,
    Secondary,
}

impl AtaBus {
    #[inline(always)]
    pub const fn io_base(self) -> u16 {
        match self {
            Self::Primary => PRIMARY_IO_BASE,
            Self::Secondary => SECONDARY_IO_BASE,
        }
    }

    #[inline(always)]
    pub const fn ctrl_base(self) -> u16 {
        match self {
            Self::Primary => PRIMARY_CTRL_BASE,
            Self::Secondary => SECONDARY_CTRL_BASE,
        }
    }
}

/// ATA Drive Selection on a bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaDriveId {
    Master,
    Slave,
}

/// Classification of device responding on an ATA bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaDriveType {
    Ata,
    Atapi,
    None,
}

/// Metadata and geometry discovered via `IDENTIFY DEVICE`.
#[derive(Debug, Clone)]
pub struct AtaDeviceInfo {
    pub bus: AtaBus,
    pub drive: AtaDriveId,
    pub drive_type: AtaDriveType,
    pub model: String,
    pub serial: String,
    pub total_sectors: u64,
}

/// Waits approximately 400 ns by reading the Alternate Status register 4 times.
#[inline(always)]
pub unsafe fn io_wait_400ns(ctrl_base: u16) {
    for _ in 0..4 {
        // SAFETY: Reading alternate status register has no destructive side effects.
        unsafe {
            inb(ctrl_base);
        }
    }
}

/// Waits for the BSY flag to clear with a timeout loop.
pub unsafe fn wait_bsy(io_base: u16, max_iter: usize) -> Result<u8, &'static str> {
    for _ in 0..max_iter {
        // SAFETY: Polling status register.
        let status = unsafe { inb(io_base + ATA_REG_STATUS_COMMAND) };
        if (status & ATA_STATUS_BSY) == 0 {
            return Ok(status);
        }
    }
    Err("ATA wait_bsy timeout")
}

/// Waits for the device to become ready for data transfer (BSY clear, DRQ set).
pub unsafe fn wait_drq(io_base: u16, max_iter: usize) -> Result<u8, &'static str> {
    for _ in 0..max_iter {
        // SAFETY: Polling status register.
        let status = unsafe { inb(io_base + ATA_REG_STATUS_COMMAND) };
        if (status & ATA_STATUS_BSY) == 0 {
            if (status & ATA_STATUS_ERR) != 0 {
                return Err("ATA device reported error (ERR set)");
            }
            if (status & ATA_STATUS_DF) != 0 {
                return Err("ATA device reported device fault (DF set)");
            }
            if (status & ATA_STATUS_DRQ) != 0 {
                return Ok(status);
            }
        }
    }
    Err("ATA wait_drq timeout")
}

/// Probes and queries an ATA drive using `IDENTIFY DEVICE` (0xEC).
pub fn identify_drive(bus: AtaBus, drive: AtaDriveId) -> Option<AtaDeviceInfo> {
    let io_base = bus.io_base();
    let ctrl_base = bus.ctrl_base();

    // SAFETY: Port I/O sequence for standard ATA IDENTIFY.
    unsafe {
        // 1. Select drive (Master = 0xA0, Slave = 0xB0)
        let drive_sel = if drive == AtaDriveId::Slave { 0xB0 } else { 0xA0 };
        outb(io_base + ATA_REG_DRIVE_SELECT, drive_sel);
        io_wait_400ns(ctrl_base);

        // 2. Clear sector count and LBA register ports
        outb(io_base + ATA_REG_SECTOR_COUNT, 0);
        outb(io_base + ATA_REG_LBA_LOW, 0);
        outb(io_base + ATA_REG_LBA_MID, 0);
        outb(io_base + ATA_REG_LBA_HIGH, 0);

        // 3. Issue IDENTIFY command (0xEC)
        outb(io_base + ATA_REG_STATUS_COMMAND, ATA_CMD_IDENTIFY);
        io_wait_400ns(ctrl_base);

        let initial_status = inb(io_base + ATA_REG_STATUS_COMMAND);
        if initial_status == 0 || initial_status == 0xFF {
            return None; // Floating bus / non-existent drive
        }

        // 4. Poll until BSY clears
        let status = match wait_bsy(io_base, 100_000) {
            Ok(s) => s,
            Err(_) => return None,
        };

        // 5. Inspect LBA Mid and LBA High to check for ATAPI signature
        let lba_mid = inb(io_base + ATA_REG_LBA_MID);
        let lba_high = inb(io_base + ATA_REG_LBA_HIGH);

        if (lba_mid == 0x14 && lba_high == 0xEB) || (lba_mid == 0x69 && lba_high == 0x96) {
            return Some(AtaDeviceInfo {
                bus,
                drive,
                drive_type: AtaDriveType::Atapi,
                model: String::from("ATAPI CD/DVD Device"),
                serial: String::from("ATAPI-ROM"),
                total_sectors: 0,
            });
        }

        // If error flag set, device does not support ATA IDENTIFY
        if (status & ATA_STATUS_ERR) != 0 {
            return None;
        }

        // 6. Wait until DRQ is ready
        if wait_drq(io_base, 100_000).is_err() {
            return None;
        }

        // 7. Read 256 words (512 bytes) of device parameters
        let mut raw_data = [0u16; 256];
        for word in raw_data.iter_mut() {
            *word = inw(io_base + ATA_REG_DATA);
        }

        // Decode Model String (words 27..47, 40 ASCII chars, big-endian in 16-bit word)
        let mut model_bytes = [0u8; 40];
        for (i, &w) in raw_data[27..47].iter().enumerate() {
            model_bytes[i * 2] = (w >> 8) as u8;
            model_bytes[i * 2 + 1] = (w & 0xFF) as u8;
        }
        let model_str = core::str::from_utf8(&model_bytes)
            .unwrap_or("ATA Hard Disk")
            .trim();

        // Decode Serial String (words 10..20, 20 ASCII chars)
        let mut serial_bytes = [0u8; 20];
        for (i, &w) in raw_data[10..20].iter().enumerate() {
            serial_bytes[i * 2] = (w >> 8) as u8;
            serial_bytes[i * 2 + 1] = (w & 0xFF) as u8;
        }
        let serial_str = core::str::from_utf8(&serial_bytes)
            .unwrap_or("UNKNOWN")
            .trim();

        // Total 28-bit LBA sectors: words 60 and 61
        let sectors_lba28 = ((raw_data[61] as u32) << 16) | (raw_data[60] as u32);

        Some(AtaDeviceInfo {
            bus,
            drive,
            drive_type: AtaDriveType::Ata,
            model: String::from(model_str),
            serial: String::from(serial_str),
            total_sectors: sectors_lba28 as u64,
        })
    }
}

/// Reads a 512-byte sector from the ATA device via 28-bit LBA PIO mode.
pub fn read_sector_pio(
    bus: AtaBus,
    drive: AtaDriveId,
    lba: u32,
    buf: &mut [u8; 512],
) -> Result<(), &'static str> {
    let io_base = bus.io_base();
    let ctrl_base = bus.ctrl_base();

    // SAFETY: Direct ATA 28-bit LBA read port I/O sequence.
    unsafe {
        wait_bsy(io_base, 100_000)?;

        let drive_head = 0xE0
            | (if drive == AtaDriveId::Slave { 0x10 } else { 0 })
            | (((lba >> 24) & 0x0F) as u8);
        outb(io_base + ATA_REG_DRIVE_SELECT, drive_head);
        io_wait_400ns(ctrl_base);

        outb(io_base + ATA_REG_SECTOR_COUNT, 1);
        outb(io_base + ATA_REG_LBA_LOW, (lba & 0xFF) as u8);
        outb(io_base + ATA_REG_LBA_MID, ((lba >> 8) & 0xFF) as u8);
        outb(io_base + ATA_REG_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);

        outb(io_base + ATA_REG_STATUS_COMMAND, ATA_CMD_READ_SECTORS);

        wait_drq(io_base, 100_000)?;

        let buf_u16 = buf.as_mut_ptr() as *mut u16;
        for i in 0..256 {
            let w = inw(io_base + ATA_REG_DATA);
            core::ptr::write_unaligned(buf_u16.add(i), w);
        }

        Ok(())
    }
}

/// Writes a 512-byte sector to the ATA device via 28-bit LBA PIO mode and flushes cache.
pub fn write_sector_pio(
    bus: AtaBus,
    drive: AtaDriveId,
    lba: u32,
    buf: &[u8; 512],
) -> Result<(), &'static str> {
    let io_base = bus.io_base();
    let ctrl_base = bus.ctrl_base();

    // SAFETY: Direct ATA 28-bit LBA write port I/O sequence.
    unsafe {
        wait_bsy(io_base, 100_000)?;

        let drive_head = 0xE0
            | (if drive == AtaDriveId::Slave { 0x10 } else { 0 })
            | (((lba >> 24) & 0x0F) as u8);
        outb(io_base + ATA_REG_DRIVE_SELECT, drive_head);
        io_wait_400ns(ctrl_base);

        outb(io_base + ATA_REG_SECTOR_COUNT, 1);
        outb(io_base + ATA_REG_LBA_LOW, (lba & 0xFF) as u8);
        outb(io_base + ATA_REG_LBA_MID, ((lba >> 8) & 0xFF) as u8);
        outb(io_base + ATA_REG_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);

        outb(io_base + ATA_REG_STATUS_COMMAND, ATA_CMD_WRITE_SECTORS);

        wait_drq(io_base, 100_000)?;

        let buf_u16 = buf.as_ptr() as *const u16;
        for i in 0..256 {
            let w = core::ptr::read_unaligned(buf_u16.add(i));
            outw(io_base + ATA_REG_DATA, w);
        }

        // Flush hardware disk write cache
        outb(io_base + ATA_REG_STATUS_COMMAND, ATA_CMD_CACHE_FLUSH);
        wait_bsy(io_base, 100_000)?;

        Ok(())
    }
}
