//! Integrate OS — ACPI Standard Description Table Header & RSDP Definitions
//!
//! Provides binary structures for the Root System Description Pointer (RSDP)
//! and standard 36-byte System Description Table (SDT) headers.

use super::types::AcpiSignature;

/// Calculates 8-bit checksum across an arbitrary memory buffer.
/// An authentic ACPI table or RSDP structure must sum to zero modulo 256.
pub fn calculate_checksum(ptr: *const u8, len: usize) -> u8 {
    let mut sum: u8 = 0;
    for i in 0..len {
        // SAFETY: Reading initialized ACPI memory space.
        let b = unsafe { core::ptr::read_volatile(ptr.add(i)) };
        sum = sum.wrapping_add(b);
    }
    sum
}

/// Root System Description Pointer (RSDP) Structure (ACPI 1.0 & 2.0+).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Rsdp {
    /// "RSD PTR " (8 bytes)
    pub signature: [u8; 8],
    /// ACPI 1.0 checksum (bytes 0..20)
    pub checksum: u8,
    /// OEM identifier string (6 bytes)
    pub oem_id: [u8; 6],
    /// 0 = ACPI 1.0, >= 2 = ACPI 2.0+
    pub revision: u8,
    /// 32-bit physical address of the RSDT
    pub rsdt_address: u32,
    /// Total length of the RSDP structure in bytes (36 in ACPI 2.0+)
    pub length: u32,
    /// 64-bit physical address of the XSDT
    pub xsdt_address: u64,
    /// Extended checksum covering all 36 bytes (ACPI 2.0+)
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

impl Rsdp {
    pub const SIGNATURE: [u8; 8] = *b"RSD PTR ";

    /// Checks if the signature matches "RSD PTR ".
    pub fn matches_signature(&self) -> bool {
        self.signature == Self::SIGNATURE
    }

    /// Validates the 8-bit checksum for ACPI 1.0 (first 20 bytes) and ACPI 2.0+ (all 36 bytes).
    pub fn verify_checksum(&self) -> bool {
        let ptr = self as *const Self as *const u8;
        // Check ACPI 1.0 20-byte table
        if calculate_checksum(ptr, 20) != 0 {
            return false;
        }

        // If ACPI 2.0+, verify extended checksum
        if self.revision >= 2 {
            let len = self.length as usize;
            if len < 36 || calculate_checksum(ptr, len) != 0 {
                return false;
            }
        }

        true
    }

    /// Returns `true` if this system adheres to ACPI 2.0+ specification.
    #[inline]
    pub fn is_acpi2(&self) -> bool {
        self.revision >= 2
    }

    /// Returns the physical address of the primary root table (XSDT if ACPI 2.0+, otherwise RSDT).
    pub fn root_table_address(&self) -> u64 {
        if self.is_acpi2() && self.xsdt_address != 0 {
            self.xsdt_address
        } else {
            self.rsdt_address as u64
        }
    }

    /// Returns OEM ID as a UTF-8 string slice.
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id).unwrap_or("UNKNOWN")
    }
}

/// Standard ACPI System Description Table (SDT) Header (36 bytes).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl SdtHeader {
    pub const HEADER_SIZE: usize = 36;

    /// Returns the 4-character table signature.
    pub fn signature(&self) -> AcpiSignature {
        AcpiSignature(self.signature)
    }

    /// Returns the total table length in bytes, including the header.
    pub fn length(&self) -> usize {
        self.length as usize
    }

    /// Returns the table specification revision.
    pub fn revision(&self) -> u8 {
        self.revision
    }

    /// Returns the OEM identifier string.
    pub fn oem_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_id).unwrap_or("UNKNOWN")
    }

    /// Returns the OEM table identifier string.
    pub fn oem_table_id_str(&self) -> &str {
        core::str::from_utf8(&self.oem_table_id).unwrap_or("UNKNOWN")
    }

    /// Verifies the checksum of the table located at physical address `phys_addr`.
    pub fn verify_checksum_at(phys_addr: u64, len: usize) -> bool {
        if len < Self::HEADER_SIZE {
            return false;
        }
        calculate_checksum(phys_addr as *const u8, len) == 0
    }

    /// Verifies the checksum of the table given its memory pointer.
    pub fn verify_checksum_ptr(ptr: *const Self) -> bool {
        // SAFETY: Reading length from header pointer.
        let len = unsafe { (*ptr).length() };
        if len < Self::HEADER_SIZE {
            return false;
        }
        calculate_checksum(ptr as *const u8, len) == 0
    }
}
