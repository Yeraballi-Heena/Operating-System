//! x86_64 Software Context Switch Assembly
//!
//! Provides `switch_context_asm` to atomically save callee-preserved registers,
//! switch stack pointers, optionally reload CR3 (address space), and restore registers.

use core::arch::global_asm;

extern "C" {
    /// Switches CPU execution from the current task context to `next_rsp`.
    ///
    /// # Safety
    /// Caller must ensure `prev_rsp` is a valid pointer to store the current stack pointer,
    /// `next_rsp` points to a valid `TaskContext` frame, and `next_cr3` is a valid PML4 base.
    pub fn switch_context_asm(prev_rsp: *mut u64, next_rsp: u64, next_cr3: u64);
}

global_asm!(
    r#"
    .section .text
    .global switch_context_asm
    .code64
switch_context_asm:
    // Arguments:
    // RDI = prev_rsp (*mut u64)
    // RSI = next_rsp (u64)
    // RDX = next_cr3 (u64)

    // 1. Save callee-preserved registers on outgoing task's stack
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    pushfq
    cli

    // 2. Store active stack pointer into outgoing task's PCB
    mov [rdi], rsp

    // 3. Switch virtual address space (CR3) if different
    mov rax, cr3
    cmp rax, rdx
    je 1f
    mov cr3, rdx
1:

    // 4. Load incoming task's stack pointer
    mov rsp, rsi

    // 5. Restore incoming task's callee-preserved registers
    popfq
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    // 6. Jump into incoming task's return address
    ret
"#
);
