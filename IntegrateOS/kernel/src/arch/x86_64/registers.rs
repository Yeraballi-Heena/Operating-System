//! x86_64 CPU Registers, Control Registers, MSRs, RFLAGS, and CPUID
//!
//! Provides strongly typed, safe wrappers around privileged x86_64 register reads
//! and isolated `unsafe` writes.

use crate::arch::x86_64::privilege::{PrivilegeLevel, SegmentSelector};

/// 10-byte Descriptor Table Register format (`GDTR` / `IDTR`).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: u64,
}

/// Control Register `CR0` flags and accessors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr0(pub u64);

impl Cr0 {
    pub const PROTECTED_MODE_ENABLE: u64 = 1 << 0;
    pub const MONITOR_COPROCESSOR: u64 = 1 << 1;
    pub const EMULATE_COPROCESSOR: u64 = 1 << 2;
    pub const TASK_SWITCHED: u64 = 1 << 3;
    pub const WRITE_PROTECT: u64 = 1 << 16;
    pub const PAGING_ENABLE: u64 = 1 << 31;

    /// Reads the current value of `CR0`.
    #[inline(always)]
    pub fn read() -> Self {
        let val: u64;
        // SAFETY: Reading CR0 in Ring 0 has no side effects.
        unsafe {
            core::arch::asm!("mov {}, cr0", out(reg) val, options(nomem, nostack, preserves_flags));
        }
        Self(val)
    }

    /// Returns `true` if all bits in `mask` are set.
    #[inline(always)]
    pub const fn contains(self, mask: u64) -> bool {
        (self.0 & mask) == mask
    }
}

/// Control Register `CR2` (Page Fault Linear Address).
pub struct Cr2;

impl Cr2 {
    /// Reads the linear address that caused the most recent page fault.
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        // SAFETY: Reading CR2 in Ring 0 has no side effects.
        unsafe {
            core::arch::asm!("mov {}, cr2", out(reg) val, options(nomem, nostack, preserves_flags));
        }
        val
    }
}

/// Control Register `CR3` (PML4 physical base address).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr3(pub u64);

impl Cr3 {
    pub const PHYS_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    /// Reads the current value of `CR3`.
    #[inline(always)]
    pub fn read() -> Self {
        let val: u64;
        // SAFETY: Reading CR3 in Ring 0 has no side effects.
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags));
        }
        Self(val)
    }

    /// Returns the 4 KiB-aligned physical address of the active PML4 table.
    #[inline(always)]
    pub const fn pml4_phys_addr(self) -> u64 {
        self.0 & Self::PHYS_ADDR_MASK
    }

    /// Writes a new PML4 physical address to `CR3`, flushing non-global TLB entries.
    ///
    /// # Safety
    /// Caller must ensure `pml4_phys` points to a valid, identity/kernel-mapped PML4 table.
    #[inline(always)]
    pub unsafe fn write(pml4_phys: u64) {
        // SAFETY: Caller guarantees valid PML4 table physical address.
        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) pml4_phys, options(nostack, preserves_flags));
        }
    }
}

/// Control Register `CR4` flags and accessors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr4(pub u64);

impl Cr4 {
    pub const PAGE_SIZE_EXTENSION: u64 = 1 << 4;
    pub const PHYSICAL_ADDRESS_EXTENSION: u64 = 1 << 5;
    pub const PAGE_GLOBAL_ENABLE: u64 = 1 << 7;
    pub const OS_FXSAVE_FXRSTOR: u64 = 1 << 9;
    pub const OS_XMM_EXCEPTION: u64 = 1 << 10;
    pub const SMEP_ENABLE: u64 = 1 << 20;
    pub const SMAP_ENABLE: u64 = 1 << 21;

    /// Reads the current value of `CR4`.
    #[inline(always)]
    pub fn read() -> Self {
        let val: u64;
        // SAFETY: Reading CR4 in Ring 0 has no side effects.
        unsafe {
            core::arch::asm!("mov {}, cr4", out(reg) val, options(nomem, nostack, preserves_flags));
        }
        Self(val)
    }

    /// Returns `true` if all bits in `mask` are set.
    #[inline(always)]
    pub const fn contains(self, mask: u64) -> bool {
        (self.0 & mask) == mask
    }
}

/// 64-bit `RFLAGS` processor state register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RFlags(pub u64);

impl RFlags {
    pub const CARRY: u64 = 1 << 0;
    pub const PARITY: u64 = 1 << 2;
    pub const ADJUST: u64 = 1 << 4;
    pub const ZERO: u64 = 1 << 6;
    pub const SIGN: u64 = 1 << 7;
    pub const TRAP: u64 = 1 << 8;
    pub const INTERRUPT_ENABLE: u64 = 1 << 9;
    pub const DIRECTION: u64 = 1 << 10;
    pub const OVERFLOW: u64 = 1 << 11;

    /// Reads the current `RFLAGS` register.
    #[inline(always)]
    pub fn read() -> Self {
        let r: u64;
        // SAFETY: Pushing RFLAGS onto the stack and popping it into a register is safe.
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) r, options(nomem, preserves_flags));
        }
        Self(r)
    }

    /// Returns `true` if hardware interrupts (`IF`) are currently enabled.
    #[inline(always)]
    pub fn interrupts_enabled() -> bool {
        (Self::read().0 & Self::INTERRUPT_ENABLE) != 0
    }
}

/// Model Specific Register (`MSR`) accessor.
#[derive(Debug, Clone, Copy)]
pub struct Msr(pub u32);

impl Msr {
    /// Extended Feature Enable Register (`IA32_EFER`).
    pub const IA32_EFER: Self = Self(0xC000_0080);
    /// System Call Target Address (`IA32_STAR`).
    pub const IA32_STAR: Self = Self(0xC000_0081);
    /// Long Mode System Call Target (`IA32_LSTAR`).
    pub const IA32_LSTAR: Self = Self(0xC000_0082);
    /// System Call Flag Mask (`IA32_FMASK`).
    pub const IA32_FMASK: Self = Self(0xC000_0084);

    pub const EFER_SYSTEM_CALL_EXTENSIONS: u64 = 1 << 0;
    pub const EFER_LONG_MODE_ENABLE: u64 = 1 << 8;
    pub const EFER_LONG_MODE_ACTIVE: u64 = 1 << 10;
    pub const EFER_NO_EXECUTE_ENABLE: u64 = 1 << 11;

    /// Reads the 64-bit value of this MSR.
    #[inline(always)]
    pub fn read(self) -> u64 {
        let low: u32;
        let high: u32;
        // SAFETY: Reading architectural MSR in Ring 0.
        unsafe {
            core::arch::asm!(
                "rdmsr",
                in("ecx") self.0,
                out("eax") low,
                out("edx") high,
                options(nomem, nostack, preserves_flags)
            );
        }
        ((high as u64) << 32) | (low as u64)
    }

    /// Writes a 64-bit value to this MSR.
    ///
    /// # Safety
    /// Caller must ensure the MSR number and value are valid for the current CPU state.
    #[inline(always)]
    pub unsafe fn write(self, val: u64) {
        let low = val as u32;
        let high = (val >> 32) as u32;
        // SAFETY: Caller guarantees valid MSR write.
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") self.0,
                in("eax") low,
                in("edx") high,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

/// Reads the active Code Segment selector (`CS`).
#[inline(always)]
pub fn read_cs() -> SegmentSelector {
    let cs: u16;
    // SAFETY: Reading CS has no side effects.
    unsafe {
        core::arch::asm!("mov {:x}, cs", out(reg) cs, options(nomem, nostack, preserves_flags));
    }
    SegmentSelector(cs)
}

/// Reads the active Data Segment selector (`DS`).
#[inline(always)]
pub fn read_ds() -> SegmentSelector {
    let ds: u16;
    // SAFETY: Reading DS has no side effects.
    unsafe {
        core::arch::asm!("mov {:x}, ds", out(reg) ds, options(nomem, nostack, preserves_flags));
    }
    SegmentSelector(ds)
}

/// Reads the active Stack Segment selector (`SS`).
#[inline(always)]
pub fn read_ss() -> SegmentSelector {
    let ss: u16;
    // SAFETY: Reading SS has no side effects.
    unsafe {
        core::arch::asm!("mov {:x}, ss", out(reg) ss, options(nomem, nostack, preserves_flags));
    }
    SegmentSelector(ss)
}

/// Reads the active Task Register selector (`TR` via `STR`).
#[inline(always)]
pub fn read_tr() -> SegmentSelector {
    let tr: u16;
    // SAFETY: Executing STR in Ring 0 has no side effects.
    unsafe {
        core::arch::asm!("str {:x}", out(reg) tr, options(nomem, nostack, preserves_flags));
    }
    SegmentSelector(tr)
}

/// Returns the Current Privilege Level (`CPL`) from `CS.RPL`.
#[inline(always)]
pub fn current_privilege_level() -> PrivilegeLevel {
    read_cs().rpl()
}

/// Reads the active `GDTR` register via `SGDT`.
#[inline(always)]
pub fn sgdt() -> DescriptorTablePointer {
    let mut gdtr = DescriptorTablePointer { limit: 0, base: 0 };
    // SAFETY: Writing 10-byte GDTR output to stack variable.
    unsafe {
        core::arch::asm!(
            "sgdt [{}]",
            in(reg) &mut gdtr,
            options(nostack, preserves_flags)
        );
    }
    gdtr
}

/// Reads the active `IDTR` register via `SIDT`.
#[inline(always)]
pub fn sidt() -> DescriptorTablePointer {
    let mut idtr = DescriptorTablePointer { limit: 0, base: 0 };
    // SAFETY: Writing 10-byte IDTR output to stack variable.
    unsafe {
        core::arch::asm!(
            "sidt [{}]",
            in(reg) &mut idtr,
            options(nostack, preserves_flags)
        );
    }
    idtr
}

/// Basic CPUID vendor & feature snapshot.
#[derive(Debug, Clone, Copy)]
pub struct CpuInfo {
    pub vendor: [u8; 12],
    pub max_basic_leaf: u32,
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
    pub has_apic: bool,
    pub has_msr: bool,
    pub has_pae: bool,
    pub has_pge: bool,
    pub has_sse2: bool,
    pub has_long_mode: bool,
    pub has_nx: bool,
}

#[inline(always)]
fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    // SAFETY: CPUID is supported on all x86_64 processors; rbx is preserved by LLVM.
    unsafe {
        core::arch::asm!(
            "mov {tmp:r}, rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "mov rbx, {tmp:r}",
            tmp = out(reg) _,
            ebx_out = out(reg) ebx,
            inout("eax") leaf => eax,
            out("ecx") ecx,
            out("edx") edx,
            options(nomem, nostack, preserves_flags)
        );
    }
    (eax, ebx, ecx, edx)
}

impl CpuInfo {
    /// Queries processor identification and architectural feature flags via `CPUID`.
    pub fn query() -> Self {
        let (max_basic, ebx0, ecx0, edx0) = cpuid(0);
        let mut vendor = [0u8; 12];
        vendor[0..4].copy_from_slice(&ebx0.to_le_bytes());
        vendor[4..8].copy_from_slice(&edx0.to_le_bytes());
        vendor[8..12].copy_from_slice(&ecx0.to_le_bytes());

        let (eax1, _ebx1, _ecx1, edx1) = cpuid(1);
        let stepping = (eax1 & 0xF) as u8;
        let base_model = ((eax1 >> 4) & 0xF) as u8;
        let base_family = ((eax1 >> 8) & 0xF) as u8;
        let ext_model = ((eax1 >> 16) & 0xF) as u8;
        let ext_family = ((eax1 >> 20) & 0xFF) as u8;

        let family = if base_family == 0xF {
            base_family.wrapping_add(ext_family)
        } else {
            base_family
        };
        let model = if base_family == 0x6 || base_family == 0xF {
            (ext_model << 4) | base_model
        } else {
            base_model
        };

        let (max_ext, _, _, _) = cpuid(0x8000_0000);
        let (has_long_mode, has_nx) = if max_ext >= 0x8000_0001 {
            let (_, _, _, ext_edx) = cpuid(0x8000_0001);
            ((ext_edx & (1 << 29)) != 0, (ext_edx & (1 << 20)) != 0)
        } else {
            (false, false)
        };

        Self {
            vendor,
            max_basic_leaf: max_basic,
            family,
            model,
            stepping,
            has_msr: (edx1 & (1 << 5)) != 0,
            has_pae: (edx1 & (1 << 6)) != 0,
            has_apic: (edx1 & (1 << 9)) != 0,
            has_pge: (edx1 & (1 << 13)) != 0,
            has_sse2: (edx1 & (1 << 26)) != 0,
            has_long_mode,
            has_nx,
        }
    }

    /// Returns the 12-character ASCII CPU vendor string (e.g. `"GenuineIntel"` or `"AuthenticAMD"`).
    pub fn vendor_str(&self) -> &str {
        core::str::from_utf8(&self.vendor).unwrap_or("UnknownCPU")
    }
}
