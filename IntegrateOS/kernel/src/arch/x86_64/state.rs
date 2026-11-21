//! x86_64 Architectural CPU State & Interrupt Context Frame
//!
//! Defines the full register snapshot saved across exceptions, hardware interrupts,
//! system calls, and process context switches.

use crate::arch::x86_64::privilege::{PrivilegeLevel, SegmentSelector};

/// Hardware stack frame automatically pushed by the x86_64 CPU on an exception or interrupt.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HardwareInterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Complete 64-bit CPU execution context saved on the kernel stack during an
/// exception, interrupt, or context switch.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuState {
    // General-purpose registers saved by software stub
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    // Vector number and hardware/synthetic error code
    pub vector: u64,
    pub error_code: u64,

    // Hardware IRETQ frame pushed by CPU
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl CpuState {
    /// Creates a zeroed `CpuState` initialized for either Kernel (Ring 0) or User (Ring 3) execution.
    pub const fn new_for_privilege(
        entry_rip: u64,
        stack_rsp: u64,
        cs: SegmentSelector,
        ss: SegmentSelector,
        enable_interrupts: bool,
    ) -> Self {
        let rflags = if enable_interrupts { 0x202 } else { 0x002 };
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            vector: 0,
            error_code: 0,
            rip: entry_rip,
            cs: cs.0 as u64,
            rflags,
            rsp: stack_rsp,
            ss: ss.0 as u64,
        }
    }

    /// Returns the privilege level (`Ring0` .. `Ring3`) of the interrupted context.
    #[inline(always)]
    pub const fn privilege_level(&self) -> PrivilegeLevel {
        PrivilegeLevel::from_u16(self.cs as u16)
    }
}
