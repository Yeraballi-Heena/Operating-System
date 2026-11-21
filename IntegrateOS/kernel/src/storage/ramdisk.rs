//! High-Performance In-Memory RAM Disk Block Device
//!
//! Provides an isolated, predictable block storage device backed by heap memory
//! for testing, caching, and fallback execution.

use alloc::vec;
use alloc::vec::Vec;

pub const SECTOR_SIZE: usize = 512;
pub const DEFAULT_RAMDISK_SECTORS: usize = 256; // 128 KiB total capacity

pub struct RamDisk {
    data: Vec<u8>,
    sectors: usize,
}

impl RamDisk {
    /// Creates a new RAM disk initialized with zeroes.
    pub fn new(sectors: usize) -> Self {
        Self {
            data: vec![0u8; sectors * SECTOR_SIZE],
            sectors,
        }
    }

    /// Reads a 512-byte sector at `lba` into `buf`.
    pub fn read_sector(&self, lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str> {
        if (lba as usize) >= self.sectors {
            return Err("RamDisk LBA out of bounds");
        }
        let offset = (lba as usize) * SECTOR_SIZE;
        buf.copy_from_slice(&self.data[offset..offset + SECTOR_SIZE]);
        Ok(())
    }

    /// Writes a 512-byte sector at `lba` from `buf`.
    pub fn write_sector(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), &'static str> {
        if (lba as usize) >= self.sectors {
            return Err("RamDisk LBA out of bounds");
        }
        let offset = (lba as usize) * SECTOR_SIZE;
        self.data[offset..offset + SECTOR_SIZE].copy_from_slice(buf);
        Ok(())
    }

    /// Returns the total number of sectors on this RAM disk.
    #[inline(always)]
    pub fn total_sectors(&self) -> u64 {
        self.sectors as u64
    }
}
