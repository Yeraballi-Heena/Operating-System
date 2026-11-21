//! Hardware Fast System Call (`syscall`/`sysret`) Initialization
//!
//! Configures architectural Model-Specific Registers (`IA32_EFER`, `IA32_STAR`,
//! `IA32_LSTAR`, `IA32_FMASK`) for hardware-accelerated user/kernel transitions.

use crate::arch::x86_64::registers::Msr;

/// Initializes hardware MSRs for the `syscall` / `sysret` instruction pair.
pub fn init_fast_syscall() {
    // 1. Enable System Call Extensions (SCE, bit 0) in IA32_EFER
    let efer = Msr::IA32_EFER.read();
    // SAFETY: Enabling architectural SCE bit in Ring 0.
    unsafe {
        Msr::IA32_EFER.write(efer | Msr::EFER_SYSTEM_CALL_EXTENSIONS);
    }

    // 2. Program IA32_STAR:
    // - Bits 47:32 = Kernel CS (0x08) and Kernel SS (0x10)
    // - Bits 63:48 = User CS/SS base (0x10 -> User SS = 0x1B, User CS = 0x23)
    let star_val = ((0x0010u64) << 48) | ((0x0008u64) << 32);
    // SAFETY: Writing valid GDT segment bases for syscall and sysret.
    unsafe {
        Msr::IA32_STAR.write(star_val);
    }

    // 3. Program IA32_FMASK:
    // Automatically mask/clear RFLAGS bits upon syscall entry:
    // - Bit 9: Interrupt Flag (IF, 0x200) -> disables interrupts until stack is switched
    // - Bit 10: Direction Flag (DF, 0x400) -> clears direction flag for cld
    // - Bit 8: Trap Flag (TF, 0x100)
    // SAFETY: Writing standard RFLAGS mask for syscall entry.
    unsafe {
        Msr::IA32_FMASK.write(0x200 | 0x400 | 0x100);
    }
}
