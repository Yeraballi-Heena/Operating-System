//! User Mode Assembly Test Payload & Execution Continuation
//!
//! Provides the User Mode Ring 3 binary payload, continuation return assembly,
//! and test outcome metrics.

use core::arch::global_asm;
use core::sync::atomic::{AtomicBool, AtomicU64};

pub static USER_SYSCALL_1_RECEIVED: AtomicBool = AtomicBool::new(false);
pub static USER_SYSCALL_2_RECEIVED: AtomicBool = AtomicBool::new(false);
pub static USER_CALCULATION_RESULT: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
pub static mut KERNEL_SAVED_RSP: u64 = 0;

extern "C" {
    pub static user_mode_code_start: u8;
    pub static user_mode_code_end: u8;
    pub fn user_mode_return_point();
    pub fn user_mode_setjmp_and_enter(user_rip: u64, user_rsp: u64);
}

/// Returns the size in bytes of the compiled user-mode binary payload.
pub fn user_payload_bytes() -> &'static [u8] {
    let start = core::ptr::addr_of!(user_mode_code_start) as usize;
    let end = core::ptr::addr_of!(user_mode_code_end) as usize;
    let len = end - start;
    // SAFETY: Symbol boundaries defined in global assembly below.
    unsafe { core::slice::from_raw_parts(start as *const u8, len) }
}

global_asm!(
    r#"
    .section .rodata
    .global user_mode_code_start
    .global user_mode_code_end
    .code64
user_mode_code_start:
    // Step 1: Verify Ring 3 CPL (low 2 bits of CS == 3)
    mov ax, cs
    and ax, 3
    cmp ax, 3
    jne 2f

    // Step 2: Perform 64-bit integer computation: (42 * 10) + 7 = 427
    mov rax, 42
    mov rbx, 10
    mul rbx
    add rax, 7

    // Step 3: Verify user stack push and pop
    push rax
    pop rdx

    // Step 4: Issue System Call 1 (SYS_USER_LOG) via int 0x80:
    // RDI = 1 (SYS_USER_LOG), RSI = 427 (computed payload)
    mov rdi, 1
    mov rsi, rdx
    int 0x80

    // Step 5: Test Privileged Instruction Protection:
    // Attempting `cli` in Ring 3 will raise #GP (vector 13).
    // The kernel #GP handler will intercept it and advance RIP+1.
    cli

    // Step 6: Test Kernel Memory Isolation:
    // Attempting to read supervisor memory (0x0010_0000) will raise #PF (vector 14).
    // The kernel #PF handler will intercept it and advance RIP+7.
    mov eax, [0x00100000]

    // Step 7: Issue System Call 2 (SYS_USER_EXIT) via int 0x80:
    // RDI = 2 (SYS_USER_EXIT), RSI = 0 (success)
    mov rdi, 2
    xor rsi, rsi
    int 0x80

1:
    pause
    jmp 1b

2:
    ud2
user_mode_code_end:

    .section .text
    .global user_mode_setjmp_and_enter
    .global user_mode_return_point
    .code64
user_mode_setjmp_and_enter:
    // Save callee-preserved registers (System V AMD64 ABI)
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    // Save active kernel RSP for iretq return continuation
    mov [rip + KERNEL_SAVED_RSP], rsp

    // Jump into User Mode trampoline
    call enter_user_mode_asm

user_mode_return_point:
    // Arrive here after kernel iretq upon SYS_USER_EXIT
    // Restore callee-preserved registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret
"#
);
