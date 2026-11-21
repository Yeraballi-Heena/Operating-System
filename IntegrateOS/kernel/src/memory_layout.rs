//! Integrate OS — Kernel Physical Memory Layout & Stack Diagnostics
//!
//! Resolves the section boundaries defined by `kernel/linker.ld` and verifies
//! the active 64-bit stack pointer (`RSP`) lies within `[__stack_bottom .. __stack_top]`.

extern "C" {
    static __kernel_start: u8;
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
    static __data_start: u8;
    static __data_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
    static __stack_bottom: u8;
    static __stack_top: u8;
    static __kernel_end: u8;
}

/// Snapshot of linker-defined kernel physical section boundaries.
#[derive(Debug, Clone, Copy)]
pub struct KernelMemoryLayout {
    pub kernel_start: usize,
    pub text_start: usize,
    pub text_end: usize,
    pub rodata_start: usize,
    pub rodata_end: usize,
    pub data_start: usize,
    pub data_end: usize,
    pub bss_start: usize,
    pub bss_end: usize,
    pub stack_bottom: usize,
    pub stack_top: usize,
    pub kernel_end: usize,
}

impl KernelMemoryLayout {
    /// Captures the linker symbol addresses for the running kernel image.
    pub fn get() -> Self {
        Self {
            kernel_start: core::ptr::addr_of!(__kernel_start) as usize,
            text_start: core::ptr::addr_of!(__text_start) as usize,
            text_end: core::ptr::addr_of!(__text_end) as usize,
            rodata_start: core::ptr::addr_of!(__rodata_start) as usize,
            rodata_end: core::ptr::addr_of!(__rodata_end) as usize,
            data_start: core::ptr::addr_of!(__data_start) as usize,
            data_end: core::ptr::addr_of!(__data_end) as usize,
            bss_start: core::ptr::addr_of!(__bss_start) as usize,
            bss_end: core::ptr::addr_of!(__bss_end) as usize,
            stack_bottom: core::ptr::addr_of!(__stack_bottom) as usize,
            stack_top: core::ptr::addr_of!(__stack_top) as usize,
            kernel_end: core::ptr::addr_of!(__kernel_end) as usize,
        }
    }

    /// Total size of the loaded kernel image in bytes.
    pub const fn total_size(&self) -> usize {
        self.kernel_end.saturating_sub(self.kernel_start)
    }

    /// Size of the kernel bootstrap stack in bytes.
    pub const fn stack_size(&self) -> usize {
        self.stack_top.saturating_sub(self.stack_bottom)
    }

    /// Verifies that the current `RSP` register lies inside the kernel stack region.
    pub fn is_rsp_within_stack(&self, rsp: usize) -> bool {
        rsp >= self.stack_bottom && rsp <= self.stack_top
    }
}

/// Reads the current 64-bit stack pointer (`RSP`).
#[inline(always)]
pub fn read_rsp() -> usize {
    let rsp: usize;
    // SAFETY: Reading RSP has no side effects.
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
    }
    rsp
}

/// Reads Control Register `CR3` (physical address of active PML4 table).
#[inline(always)]
pub fn read_cr3() -> u64 {
    let cr3: u64;
    // SAFETY: Reading CR3 in Ring 0 has no side effects.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}
