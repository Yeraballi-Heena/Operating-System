//! CPU Exception Decoding & Architectural Fault Diagnostics
//!
//! Provides binary decoding of x86_64 hardware exception error codes:
//! - Page Fault (`#PF`) bitmask interpretation (Present, Write, User, Reserved, Instruction Fetch)
//! - General Protection Fault (`#GP`) segment selector parsing (GDT/LDT, IDT, External, Index)
//! - Architectural exception vector taxonomy

/// Architectural table indicator for `#GP` and `#TS` error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableIndicator {
    Gdt,
    Ldt,
}

/// Decoded Page Fault (`#PF`) hardware error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFaultError {
    /// True if the fault was caused by a page-protection violation; false if caused by a non-present page.
    pub present: bool,
    /// True if the access causing the fault was a write; false if a read.
    pub write: bool,
    /// True if the access occurred in User Mode (Ring 3); false if in Supervisor Mode (Ring 0).
    pub user_mode: bool,
    /// True if the fault was caused by a reserved bit set to 1 in a paging-structure entry.
    pub reserved_write: bool,
    /// True if the fault was caused by an instruction fetch (NX / Execute-Disable violation).
    pub instruction_fetch: bool,
    /// True if the fault was caused by a protection-key violation.
    pub protection_key: bool,
}

impl PageFaultError {
    /// Decodes raw 64-bit hardware error code pushed onto the stack by the CPU.
    pub const fn from_raw(code: u64) -> Self {
        Self {
            present: (code & (1 << 0)) != 0,
            write: (code & (1 << 1)) != 0,
            user_mode: (code & (1 << 2)) != 0,
            reserved_write: (code & (1 << 3)) != 0,
            instruction_fetch: (code & (1 << 4)) != 0,
            protection_key: (code & (1 << 5)) != 0,
        }
    }
}

/// Decoded General Protection Fault (`#GP`) hardware error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralProtectionError {
    /// True if the exception occurred during delivery of an external event/interrupt.
    pub external: bool,
    /// True if the selector index refers to a gate descriptor in the IDT.
    pub idt: bool,
    /// Identifies whether the selector references the GDT or LDT (when `idt` is false).
    pub table: TableIndicator,
    /// The selector index into the specified descriptor table.
    pub selector_index: u16,
}

impl GeneralProtectionError {
    /// Decodes raw 64-bit hardware error code pushed onto the stack by the CPU.
    pub const fn from_raw(code: u64) -> Self {
        let external = (code & (1 << 0)) != 0;
        let idt = (code & (1 << 1)) != 0;
        let table = if (code & (1 << 2)) != 0 {
            TableIndicator::Ldt
        } else {
            TableIndicator::Gdt
        };
        let selector_index = ((code >> 3) & 0x1FFF) as u16;

        Self {
            external,
            idt,
            table,
            selector_index,
        }
    }
}

/// Returns the standard architectural name of an x86_64 exception vector.
pub const fn exception_name(vector: u8) -> &'static str {
    match vector {
        0 => "#DE (Divide Error)",
        1 => "#DB (Debug Exception)",
        2 => "NMI (Non-Maskable Interrupt)",
        3 => "#BP (Breakpoint)",
        4 => "#OF (Overflow)",
        5 => "#BR (BOUND Range Exceeded)",
        6 => "#UD (Invalid Opcode)",
        7 => "#NM (Device Not Available)",
        8 => "#DF (Double Fault)",
        9 => "Coprocessor Segment Overrun",
        10 => "#TS (Invalid TSS)",
        11 => "#NP (Segment Not Present)",
        12 => "#SS (Stack-Segment Fault)",
        13 => "#GP (General Protection Fault)",
        14 => "#PF (Page Fault)",
        16 => "#MF (x87 FPU Floating-Point Error)",
        17 => "#AC (Alignment Check)",
        18 => "#MC (Machine Check)",
        19 => "#XM (SIMD Floating-Point Exception)",
        20 => "#VE (Virtualization Exception)",
        21 => "#CP (Control Protection Exception)",
        30 => "#SX (Security Exception)",
        _ => "Unknown CPU Exception",
    }
}
