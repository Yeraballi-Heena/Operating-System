//! Integrate OS — Root System Description Pointer (RSDP) Discovery
//!
//! Scans the Extended BIOS Data Area (EBDA) and the physical BIOS read-only memory range
//! (0x000E0000 .. 0x000FFFFF) on 16-byte aligned boundaries to locate the ACPI RSDP.

use super::sdt::Rsdp;
use crate::klog_info;

/// Physical memory range boundaries for BIOS read-only area.
pub const BIOS_SEARCH_START: usize = 0x000E_0000;
pub const BIOS_SEARCH_END: usize = 0x000F_FFFF;

/// Searches physical memory for the ACPI Root System Description Pointer (RSDP).
pub fn find_rsdp() -> Option<Rsdp> {
    // 1. Scan Extended BIOS Data Area (EBDA) if present
    // SAFETY: Reading 16-bit EBDA segment pointer at physical address 0x040E.
    let ebda_segment = unsafe { core::ptr::read_volatile(0x040E as *const u16) };
    let ebda_base = (ebda_segment as usize) << 4;

    if ebda_base >= 0x80000 && ebda_base < 0xA0000 {
        if let Some(rsdp) = scan_memory_range(ebda_base, ebda_base + 1024) {
            klog_info!(
                "[acpi] Located RSDP in Extended BIOS Data Area (EBDA) at 0x{:08X}",
                ebda_base
            );
            return Some(rsdp);
        }
    }

    // 2. Scan BIOS Read-Only Memory space [0x000E0000 .. 0x000FFFFF]
    if let Some(rsdp) = scan_memory_range(BIOS_SEARCH_START, BIOS_SEARCH_END) {
        klog_info!("[acpi] Located RSDP in main BIOS memory range [0x000E0000 .. 0x000FFFFF]");
        return Some(rsdp);
    }

    None
}

/// Scans a contiguous physical memory range in 16-byte steps for the RSDP signature.
fn scan_memory_range(start: usize, end: usize) -> Option<Rsdp> {
    let mut addr = start;
    while addr + core::mem::size_of::<Rsdp>() <= end {
        // SAFETY: The lower 1 MiB is mapped and accessible in Ring 0.
        let candidate = unsafe { core::ptr::read_volatile(addr as *const Rsdp) };
        if candidate.matches_signature() && candidate.verify_checksum() {
            return Some(candidate);
        }
        addr += 16;
    }
    None
}
