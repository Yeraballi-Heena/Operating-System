//! x86_64 Global Descriptor Table (GDT) & Segment Selector Management
//!
//! Defines the architectural 64-bit GDT layout supporting both Kernel Mode (Ring 0)
//! and User Mode (Ring 3) execution, plus the 16-byte 64-bit Task State Segment (TSS)
//! system descriptor:
//!
//! | Index | Offset | Selector | DPL    | Description                         |
//! |-------|--------|----------|--------|-------------------------------------|
//! | 0     | `0x00` | `0x00`   | -      | Null Descriptor                     |
//! | 1     | `0x08` | `0x08`   | Ring 0 | Kernel 64-Bit Code Segment (`RX`)   |
//! | 2     | `0x10` | `0x10`   | Ring 0 | Kernel Data Segment (`RW`)          |
//! | 3     | `0x18` | `0x1B`   | Ring 3 | User Data Segment (`RW`)            |
//! | 4     | `0x20` | `0x23`   | Ring 3 | User 64-Bit Code Segment (`RX`)     |
//! | 5..6  | `0x28` | `0x28`   | Ring 0 | 64-Bit Available TSS (16-byte)      |

use crate::arch::x86_64::privilege::{PrivilegeLevel, SegmentSelector};
use crate::arch::x86_64::registers::DescriptorTablePointer;
use crate::arch::x86_64::tss;
use core::mem::size_of;

/// Kernel 64-bit Code Segment selector (`Index 1, Ring 0 = 0x08`).
pub const KERNEL_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, PrivilegeLevel::Ring0);

/// Kernel Data Segment selector (`Index 2, Ring 0 = 0x10`).
pub const KERNEL_DATA_SELECTOR: SegmentSelector = SegmentSelector::new(2, PrivilegeLevel::Ring0);

/// User Data Segment selector (`Index 3, Ring 3 = 0x1B`).
pub const USER_DATA_SELECTOR: SegmentSelector = SegmentSelector::new(3, PrivilegeLevel::Ring3);

/// User 64-bit Code Segment selector (`Index 4, Ring 3 = 0x23`).
pub const USER_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(4, PrivilegeLevel::Ring3);

/// 64-bit Task State Segment selector (`Index 5, Ring 0 = 0x28`).
pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(5, PrivilegeLevel::Ring0);

/// Number of 64-bit slots in the GDT (5 standard descriptors + 2 slots for 16-byte TSS).
pub const GDT_ENTRY_COUNT: usize = 7;

/// Expected `GDTR.limit` value (`7 * 8 - 1 = 55 = 0x37`).
pub const EXPECTED_GDT_LIMIT: u16 = (GDT_ENTRY_COUNT * size_of::<u64>() - 1) as u16;

#[repr(C, align(16))]
struct GdtTable {
    entries: [u64; GDT_ENTRY_COUNT],
}

impl GdtTable {
    const NULL: u64 = 0x0000_0000_0000_0000;
    /// Kernel 64-bit code segment: Long Mode (`L=1`), `D=0`, Present (`P=1`), `DPL=0`, Exec/Read (`0x9A`).
    const KERNEL_CODE: u64 = 0x00AF_9A00_0000_FFFF;
    /// Kernel data segment: Present (`P=1`), `DPL=0`, Read/Write (`0x92`).
    const KERNEL_DATA: u64 = 0x00CF_9200_0000_FFFF;
    /// User data segment: Present (`P=1`), `DPL=3`, Read/Write (`0xF2`).
    const USER_DATA: u64 = 0x00CF_F200_0000_FFFF;
    /// User 64-bit code segment: Long Mode (`L=1`), `D=0`, Present (`P=1`), `DPL=3`, Exec/Read (`0xFA`).
    const USER_CODE: u64 = 0x00AF_FA00_0000_FFFF;

    const fn new() -> Self {
        Self {
            entries: [
                Self::NULL,
                Self::KERNEL_CODE,
                Self::KERNEL_DATA,
                Self::USER_DATA,
                Self::USER_CODE,
                0, // TSS descriptor low 64 bits (populated at runtime)
                0, // TSS descriptor high 64 bits (populated at runtime)
            ],
        }
    }
}

static mut GDT: GdtTable = GdtTable::new();

/// Encodes a 64-bit TSS base address and 16-bit byte limit into a 16-byte (`(low, high)`)
/// x86_64 System Segment Descriptor (`Type = 0x9`, Available 64-bit TSS, `DPL = 0`, `P = 1`).
#[inline]
const fn encode_tss_descriptor(base: u64, limit: u16) -> (u64, u64) {
    let mut low: u64 = 0;
    // Bits 15..0: Limit[15:0]
    low |= (limit as u64) & 0xFFFF;
    // Bits 39..16: Base[23:0]
    low |= (base & 0x00FF_FFFF) << 16;
    // Bits 47..40: Access byte = 0x89 (Present=1, DPL=00, S=0, Type=1001 Available 64-bit TSS)
    low |= 0x89u64 << 40;
    // Bits 63..56: Base[31:24]
    low |= ((base >> 24) & 0xFF) << 56;

    // High 64 bits: Bits 31..0 = Base[63:32], Bits 63..32 = Reserved (0)
    let high: u64 = (base >> 32) & 0xFFFF_FFFF;
    (low, high)
}

/// Initializes the 64-bit TSS and GDT, loads `GDTR` (`lgdt`), reloads `CS`, `DS`, `ES`,
/// `FS`, `GS`, `SS`, and loads the Task Register (`ltr`).
pub fn init() {
    let (tss_base, tss_limit) = tss::init();
    let (tss_low, tss_high) = encode_tss_descriptor(tss_base, tss_limit);

    // SAFETY: Populating static GDT during single-core initialization with interrupts disabled.
    let gdtr = unsafe {
        GDT.entries[5] = tss_low;
        GDT.entries[6] = tss_high;

        DescriptorTablePointer {
            limit: EXPECTED_GDT_LIMIT,
            base: core::ptr::addr_of!(GDT.entries) as u64,
        }
    };

    // SAFETY: Loading validated 64-bit GDT, reloading kernel segment selectors, and loading TR.
    unsafe {
        load_gdt_and_segments(
            &gdtr,
            KERNEL_CODE_SELECTOR.0,
            KERNEL_DATA_SELECTOR.0,
            TSS_SELECTOR.0,
        );
    }
}

/// Returns the expected base address of the static GDT table.
pub fn gdt_base_addr() -> u64 {
    core::ptr::addr_of!(GDT) as u64
}

/// Low-level routine that executes `lgdt`, reloads `CS` via `retfq`, reloads
/// data/stack segments (`DS`, `ES`, `FS`, `GS`, `SS`), and loads the Task Register (`ltr`).
///
/// # Safety
/// `gdtr` must point to a valid 64-bit GDT containing `code_sel`, `data_sel`, and `tss_sel`.
#[inline(never)]
unsafe fn load_gdt_and_segments(
    gdtr: &DescriptorTablePointer,
    code_sel: u16,
    data_sel: u16,
    tss_sel: u16,
) {
    // SAFETY: Validated GDT and selector layout.
    unsafe {
        core::arch::asm!(
            "lgdt [{gdtr}]",
            // Reload CS using 64-bit far return (retfq)
            "push {code_sel:r}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            // Reload data and stack segment registers
            "mov ds, {data_sel:x}",
            "mov es, {data_sel:x}",
            "mov ss, {data_sel:x}",
            // Zero FS and GS selectors (MSR base addresses remain intact)
            "xor {tmp:e}, {tmp:e}",
            "mov fs, {tmp:x}",
            "mov gs, {tmp:x}",
            // Load 64-bit Task State Segment selector into TR
            "ltr {tss_sel:x}",
            gdtr = in(reg) gdtr,
            code_sel = in(reg) code_sel as u64,
            data_sel = in(reg) data_sel,
            tss_sel = in(reg) tss_sel,
            tmp = out(reg) _,
            options(preserves_flags)
        );
    }
}
