//! Integrate OS — PCI BDF (Bus/Device/Function) Addressing
//!
//! Provides strongly typed PCI address coordinates and configuration space read/write primitives.

use alloc::string::String;
use core::fmt;
use crate::arch::x86_64::io::{pci_read_u32, pci_write_u32};

/// 3-tuple representing a unique physical PCI device function: Bus (0..255), Device (0..31), Function (0..7).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    pub const fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device: device & 0x1F,
            function: function & 0x07,
        }
    }

    /// Reads a 32-bit dword from configuration space at `offset`.
    #[inline]
    pub fn read_u32(&self, offset: u8) -> u32 {
        // SAFETY: Reading 32-bit aligned PCI configuration dword.
        unsafe { pci_read_u32(self.bus, self.device, self.function, offset) }
    }

    /// Writes a 32-bit dword to configuration space at `offset`.
    #[inline]
    pub fn write_u32(&self, offset: u8, val: u32) {
        // SAFETY: Writing 32-bit aligned PCI configuration dword.
        unsafe { pci_write_u32(self.bus, self.device, self.function, offset, val) }
    }

    /// Reads a 16-bit word from configuration space at `offset`.
    #[inline]
    pub fn read_u16(&self, offset: u8) -> u16 {
        let dword = self.read_u32(offset & 0xFC);
        let shift = (offset & 0x02) * 8;
        ((dword >> shift) & 0xFFFF) as u16
    }

    /// Writes a 16-bit word to configuration space at `offset` using read-modify-write.
    #[inline]
    pub fn write_u16(&self, offset: u8, val: u16) {
        let aligned = offset & 0xFC;
        let shift = (offset & 0x02) * 8;
        let mask = !(0xFFFFu32 << shift);
        let cur = self.read_u32(aligned);
        let new_val = (cur & mask) | ((val as u32) << shift);
        self.write_u32(aligned, new_val);
    }

    /// Reads an 8-bit byte from configuration space at `offset`.
    #[inline]
    pub fn read_u8(&self, offset: u8) -> u8 {
        let dword = self.read_u32(offset & 0xFC);
        let shift = (offset & 0x03) * 8;
        ((dword >> shift) & 0xFF) as u8
    }

    /// Writes an 8-bit byte to configuration space at `offset` using read-modify-write.
    #[inline]
    pub fn write_u8(&self, offset: u8, val: u8) {
        let aligned = offset & 0xFC;
        let shift = (offset & 0x03) * 8;
        let mask = !(0xFFu32 << shift);
        let cur = self.read_u32(aligned);
        let new_val = (cur & mask) | ((val as u32) << shift);
        self.write_u32(aligned, new_val);
    }

    /// Formats the address into standard BDF notation: `"00:03.0"`.
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(7);
        format_hex_u8(&mut s, self.bus);
        s.push(':');
        format_hex_u8(&mut s, self.device);
        s.push('.');
        s.push((b'0' + self.function) as char);
        s
    }

    /// Alias for `to_string` to format BDF notation.
    pub fn format(&self) -> String {
        self.to_string()
    }
}

impl fmt::Debug for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}.{:x}",
            self.bus, self.device, self.function
        )
    }
}

fn format_hex_u8(s: &mut String, val: u8) {
    let hex = b"0123456789abcdef";
    s.push(hex[(val >> 4) as usize] as char);
    s.push(hex[(val & 0x0F) as usize] as char);
}
