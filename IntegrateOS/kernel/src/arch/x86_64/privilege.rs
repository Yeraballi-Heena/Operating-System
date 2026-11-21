//! x86_64 Hardware Privilege Levels (Rings 0..3) and Segment Selectors
//!
//! Preserves the foundational kernel/user privilege separation principle:
//! Ring 0 (`PrivilegeLevel::Ring0`) executes privileged kernel instructions;
//! Ring 3 (`PrivilegeLevel::Ring3`) restricts user applications from direct
//! hardware, descriptor table, or kernel memory access.

/// x86_64 2-bit Current / Requested Privilege Level (`CPL` / `RPL` / `DPL`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    /// Ring 0 — Kernel mode (full hardware and instruction privilege).
    Ring0 = 0,
    /// Ring 1 — Unused in standard 64-bit paging model.
    Ring1 = 1,
    /// Ring 2 — Unused in standard 64-bit paging model.
    Ring2 = 2,
    /// Ring 3 — User mode (unprivileged application execution).
    Ring3 = 3,
}

impl PrivilegeLevel {
    /// Converts a low 2-bit integer (`0..=3`) into a `PrivilegeLevel`.
    #[inline(always)]
    pub const fn from_u16(val: u16) -> Self {
        match val & 0b11 {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            _ => Self::Ring3,
        }
    }

    /// Returns `true` if this privilege level is Ring 0 (Kernel mode).
    #[inline(always)]
    pub const fn is_kernel(self) -> bool {
        matches!(self, Self::Ring0)
    }

    /// Returns `true` if this privilege level is Ring 3 (User mode).
    #[inline(always)]
    pub const fn is_user(self) -> bool {
        matches!(self, Self::Ring3)
    }
}

/// 16-bit x86_64 Segment Selector (`[Index: 15..3 | TI: 2 | RPL: 1..0]`).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    /// Creates a new GDT segment selector (`TI = 0`) for the given descriptor `index` and `rpl`.
    #[inline(always)]
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self((index << 3) | (rpl as u16))
    }

    /// Returns the GDT descriptor index (`bits 15..3`).
    #[inline(always)]
    pub const fn index(self) -> u16 {
        self.0 >> 3
    }

    /// Returns the Requested Privilege Level (`bits 1..0`).
    #[inline(always)]
    pub const fn rpl(self) -> PrivilegeLevel {
        PrivilegeLevel::from_u16(self.0)
    }
}
