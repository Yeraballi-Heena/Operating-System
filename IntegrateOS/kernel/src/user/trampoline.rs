//! x86_64 User Mode Transition Trampoline
//!
//! Provides the assembly routine `enter_user_mode_asm` executing the `iretq`
//! privilege boundary transition from Ring 0 to Ring 3.

use core::arch::global_asm;

extern "C" {
    /// Jumps to user space (Ring 3) via `iretq`.
    pub fn enter_user_mode_asm(user_entry_rip: u64, user_stack_rsp: u64) -> !;
}

global_asm!(
    r#"
    .section .text
    .global enter_user_mode_asm
    .code64
enter_user_mode_asm:
    // Arguments passed via System V AMD64 ABI:
    // RDI = user_entry_rip
    // RSI = user_stack_rsp

    // Load User Data Segment selector (0x1B = index 3 | RPL 3) into data segment registers
    mov ax, 0x1B
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // Construct the 5-quadword hardware exception/interrupt return frame for `iretq`:
    // Stack layout (high to low):
    // [RSP + 32] = SS     (0x1B, User Data Selector, RPL=3)
    // [RSP + 24] = RSP    (user_stack_rsp)
    // [RSP + 16] = RFLAGS (0x202: Interrupts Enabled IF=1, IOPL=0)
    // [RSP + 8]  = CS     (0x23, User Code Selector, RPL=3)
    // [RSP + 0]  = RIP    (user_entry_rip)
    push 0x1B
    push rsi
    push 0x202
    push 0x23
    push rdi

    // Execute atomic privilege transition: Ring 0 -> Ring 3
    iretq
"#
);
