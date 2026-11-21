//! Integrate OS — ACPI Description Table Parsers (RSDT, XSDT, FADT, MADT)
//!
//! Parses the root description table pointer array, discovers specialized ACPI tables,
//! and decodes the Fixed ACPI Description Table (FADT) and Multiple APIC Description Table (MADT).

use alloc::vec::Vec;
use super::sdt::{Rsdp, SdtHeader};
use super::types::{AcpiError, AcpiSignature, GenericAddress};
use crate::{klog_info, klog_ok};

/// Represents an enumerated ACPI description table reference.
#[derive(Debug, Clone, Copy)]
pub struct TableEntry {
    pub signature: AcpiSignature,
    pub phys_addr: u64,
    pub length: usize,
    pub revision: u8,
}

/// Root ACPI Tables Dispatcher managing RSDT or XSDT.
pub struct AcpiTables {
    tables: Vec<TableEntry>,
    root_signature: AcpiSignature,
}

impl AcpiTables {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            root_signature: AcpiSignature::RSDT,
        }
    }

    /// Parses the root ACPI table from an authenticated RSDP structure.
    pub fn parse_from_rsdp(&mut self, rsdp: &Rsdp) -> Result<(), AcpiError> {
        self.tables.clear();
        let root_addr = rsdp.root_table_address();
        if root_addr == 0 {
            return Err(AcpiError::TableNotFound);
        }

        // SAFETY: Reading root table header from physical memory.
        let header = unsafe { core::ptr::read_unaligned(root_addr as *const SdtHeader) };
        let total_len = header.length();
        if !SdtHeader::verify_checksum_at(root_addr, total_len) {
            return Err(AcpiError::ChecksumMismatch);
        }

        self.root_signature = header.signature();
        let is_xsdt = self.root_signature == AcpiSignature::XSDT;

        if total_len < SdtHeader::HEADER_SIZE {
            return Err(AcpiError::InvalidHeader);
        }

        let entries_bytes = total_len - SdtHeader::HEADER_SIZE;
        let entries_base = root_addr + SdtHeader::HEADER_SIZE as u64;

        if is_xsdt {
            // XSDT: Array of 64-bit pointers
            let count = entries_bytes / 8;
            for i in 0..count {
                // SAFETY: Reading 64-bit pointer array entries (unaligned safe).
                let table_phys = unsafe {
                    core::ptr::read_unaligned((entries_base + (i * 8) as u64) as *const u64)
                };
                self.probe_table(table_phys);
            }
        } else {
            // RSDT: Array of 32-bit pointers
            let count = entries_bytes / 4;
            for i in 0..count {
                // SAFETY: Reading 32-bit pointer array entries (unaligned safe).
                let table_phys = unsafe {
                    core::ptr::read_unaligned((entries_base + (i * 4) as u64) as *const u32)
                } as u64;
                self.probe_table(table_phys);
            }
        }

        klog_ok!(
            "[acpi] Successfully enumerated {} ACPI tables via {}",
            self.tables.len(),
            self.root_signature
        );

        Ok(())
    }

    /// Probes and validates a table pointer, recording valid entries.
    fn probe_table(&mut self, phys_addr: u64) {
        if phys_addr == 0 {
            return;
        }

        // SAFETY: Reading table header from physical address.
        let header = unsafe { core::ptr::read_unaligned(phys_addr as *const SdtHeader) };
        let len = header.length();
        if SdtHeader::verify_checksum_at(phys_addr, len) {
            let sig = header.signature();
            let rev = header.revision();

            klog_info!(
                "[acpi]   Table: [{}] at 0x{:08X} (len: {} bytes, rev: {}, OEM: '{}')",
                sig,
                phys_addr,
                len,
                rev,
                header.oem_id_str()
            );

            self.tables.push(TableEntry {
                signature: sig,
                phys_addr,
                length: len,
                revision: rev,
            });
        }
    }

    /// Finds a table by its 4-character signature.
    pub fn find(&self, sig: AcpiSignature) -> Option<&TableEntry> {
        self.tables.iter().find(|t| t.signature == sig)
    }

    /// Returns a slice of all enumerated table entries.
    pub fn entries(&self) -> &[TableEntry] {
        &self.tables
    }
}

/// Fixed ACPI Description Table (FADT).
#[derive(Debug, Clone, Copy)]
pub struct Fadt {
    pub dsdt_address: u64,
    pub smi_cmd_port: u16,
    pub acpi_enable_val: u8,
    pub acpi_disable_val: u8,
    pub pm1a_cnt_blk: u16,
    pub pm1b_cnt_blk: u16,
    pub pm1a_evt_blk: u16,
    pub pm_tmr_blk: u16,
    pub reset_reg: GenericAddress,
    pub reset_value: u8,
    pub flags: u32,
    pub century_reg: u8,
}

impl Fadt {
    /// Parses the FADT from its physical memory address.
    pub fn parse(entry: &TableEntry) -> Result<Self, AcpiError> {
        if entry.signature != AcpiSignature::FADT || entry.length < 116 {
            return Err(AcpiError::InvalidHeader);
        }

        let ptr = entry.phys_addr as *const u8;

        // SAFETY: Reading validated FADT table memory fields via offsets.
        unsafe {
            let dsdt_32 = core::ptr::read_unaligned(ptr.add(40) as *const u32) as u64;
            let smi_cmd = core::ptr::read_unaligned(ptr.add(48) as *const u32) as u16;
            let acpi_en = core::ptr::read_volatile(ptr.add(52));
            let acpi_dis = core::ptr::read_volatile(ptr.add(53));
            let pm1a_evt = core::ptr::read_unaligned(ptr.add(56) as *const u32) as u16;
            let pm1a_cnt = core::ptr::read_unaligned(ptr.add(64) as *const u32) as u16;
            let pm1b_cnt = core::ptr::read_unaligned(ptr.add(68) as *const u32) as u16;
            let pm_tmr = core::ptr::read_unaligned(ptr.add(76) as *const u32) as u16;
            let century = core::ptr::read_volatile(ptr.add(108));
            let flags = core::ptr::read_unaligned(ptr.add(112) as *const u32);

            // Reset register is at offset 116 if table length >= 129
            let (reset_reg, reset_val) = if entry.length >= 129 {
                let r_reg = core::ptr::read_unaligned(ptr.add(116) as *const GenericAddress);
                let r_val = core::ptr::read_volatile(ptr.add(128));
                (r_reg, r_val)
            } else {
                (GenericAddress::empty(), 0)
            };

            // 64-bit X_DSDT at offset 140 if ACPI 2.0+
            let dsdt_address = if entry.length >= 148 {
                let x_dsdt = core::ptr::read_unaligned(ptr.add(140) as *const u64);
                if x_dsdt != 0 { x_dsdt } else { dsdt_32 }
            } else {
                dsdt_32
            };

            Ok(Self {
                dsdt_address,
                smi_cmd_port: smi_cmd,
                acpi_enable_val: acen_fix(acpi_en),
                acpi_disable_val: acpi_dis,
                pm1a_cnt_blk: pm1a_cnt,
                pm1b_cnt_blk: pm1b_cnt,
                pm1a_evt_blk: pm1a_evt,
                pm_tmr_blk: pm_tmr,
                reset_reg,
                reset_value: reset_val,
                flags,
                century_reg: century,
            })
        }
    }

    /// Returns `true` if hardware supports ACPI reset register mechanism (bit 10 of flags).
    pub fn supports_acpi_reset(&self) -> bool {
        (self.flags & (1 << 10)) != 0 && self.reset_reg.address != 0
    }
}

const fn acen_fix(v: u8) -> u8 {
    v
}

/// Multiple APIC Description Table (MADT) Summary.
#[derive(Debug, Clone, Copy)]
pub struct Madt {
    pub local_apic_address: u32,
    pub flags: u32,
    pub num_local_apics: u8,
    pub num_io_apics: u8,
    pub num_iso: u8,
}

impl Madt {
    pub fn parse(entry: &TableEntry) -> Result<Self, AcpiError> {
        if entry.signature != AcpiSignature::MADT || entry.length < 44 {
            return Err(AcpiError::InvalidHeader);
        }

        let ptr = entry.phys_addr as *const u8;

        // SAFETY: Reading MADT base fields.
        let (lapic_addr, flags) = unsafe {
            let lapic = core::ptr::read_unaligned(ptr.add(36) as *const u32);
            let fl = core::ptr::read_unaligned(ptr.add(40) as *const u32);
            (lapic, fl)
        };

        let mut num_lapics = 0;
        let mut num_ioapics = 0;
        let mut num_iso = 0;

        let mut offset = 44;
        while offset + 2 <= entry.length {
            // SAFETY: Iterating variable-length MADT entry structures.
            unsafe {
                let entry_type = core::ptr::read_volatile(ptr.add(offset));
                let entry_len = core::ptr::read_volatile(ptr.add(offset + 1)) as usize;

                if entry_len == 0 || offset + entry_len > entry.length {
                    break;
                }

                match entry_type {
                    0 => num_lapics += 1,  // Processor Local APIC
                    1 => num_ioapics += 1, // I/O APIC
                    2 => num_iso += 1,     // Interrupt Source Override
                    _ => {}
                }

                offset += entry_len;
            }
        }

        Ok(Self {
            local_apic_address: lapic_addr,
            flags,
            num_local_apics: num_lapics,
            num_io_apics: num_ioapics,
            num_iso,
        })
    }
}
