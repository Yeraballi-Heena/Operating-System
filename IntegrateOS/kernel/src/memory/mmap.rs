//! BIOS E820 / Multiboot Firmware Memory Map Parser
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Iterates through memory map entries passed by GRUB in the Multiboot structure,
//! distinguishing usable RAM from reserved, ACPI, and firmware memory ranges.

/// Entry type indicating standard usable RAM.
pub const MMAP_TYPE_USABLE: u32 = 1;
/// Entry type indicating reserved hardware/ROM memory.
pub const MMAP_TYPE_RESERVED: u32 = 2;
/// Entry type indicating ACPI reclaimable memory.
pub const MMAP_TYPE_ACPI_RECLAIMABLE: u32 = 3;
/// Entry type indicating ACPI NVS memory.
pub const MMAP_TYPE_ACPI_NVS: u32 = 4;
/// Entry type indicating bad/defective memory.
pub const MMAP_TYPE_BAD: u32 = 5;

/// Raw Multiboot1 E820 memory map entry structure.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MultibootMmapEntry {
    pub size: u32,
    pub base_addr: u64,
    pub length: u64,
    pub entry_type: u32,
}

/// Normalized memory region descriptor.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub region_type: u32,
}

impl MemoryRegion {
    pub const fn is_usable(&self) -> bool {
        self.region_type == MMAP_TYPE_USABLE
    }

    pub const fn end(&self) -> u64 {
        self.base + self.length
    }

    pub const fn type_str(&self) -> &'static str {
        match self.region_type {
            MMAP_TYPE_USABLE => "Usable RAM",
            MMAP_TYPE_RESERVED => "Reserved",
            MMAP_TYPE_ACPI_RECLAIMABLE => "ACPI Reclaimable",
            MMAP_TYPE_ACPI_NVS => "ACPI NVS",
            MMAP_TYPE_BAD => "Bad RAM",
            _ => "Unknown",
        }
    }
}

pub const MAX_MEMORY_REGIONS: usize = 32;

/// System-wide physical memory overview parsed from firmware.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalMemorySummary {
    pub regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    pub region_count: usize,
    pub total_usable_bytes: u64,
    pub total_reserved_bytes: u64,
    pub highest_usable_address: u64,
}

impl PhysicalMemorySummary {
    pub const fn empty() -> Self {
        Self {
            regions: [MemoryRegion {
                base: 0,
                length: 0,
                region_type: 0,
            }; MAX_MEMORY_REGIONS],
            region_count: 0,
            total_usable_bytes: 0,
            total_reserved_bytes: 0,
            highest_usable_address: 0,
        }
    }
}

/// Parses the Multiboot E820 memory map buffer.
///
/// # Safety
/// `mmap_addr` must point to valid identity-mapped memory of `mmap_length` bytes.
pub unsafe fn parse_mmap(mmap_addr: u64, mmap_length: u32) -> PhysicalMemorySummary {
    let mut summary = PhysicalMemorySummary::empty();
    if mmap_addr == 0 || mmap_length == 0 {
        return summary;
    }

    let mut current_offset: u32 = 0;

    while current_offset < mmap_length && summary.region_count < MAX_MEMORY_REGIONS {
        let entry_ptr = (mmap_addr + current_offset as u64) as *const MultibootMmapEntry;
        // SAFETY: Reading unaligned entry inside mmap bounds.
        let entry = unsafe { core::ptr::read_unaligned(entry_ptr) };

        let region = MemoryRegion {
            base: entry.base_addr,
            length: entry.length,
            region_type: entry.entry_type,
        };

        if region.is_usable() {
            summary.total_usable_bytes += region.length;
            let end_addr = region.end();
            if end_addr > summary.highest_usable_address {
                summary.highest_usable_address = end_addr;
            }
        } else {
            summary.total_reserved_bytes += region.length;
        }

        summary.regions[summary.region_count] = region;
        summary.region_count += 1;

        // In Multiboot1, entry size field is the size of following fields (typically 20 bytes).
        // Total step is entry.size + 4.
        current_offset += entry.size + 4;
    }

    summary
}
