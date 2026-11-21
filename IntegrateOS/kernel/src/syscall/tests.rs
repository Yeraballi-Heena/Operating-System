//! Milestone D — System Call ABI & Pointer Security Verification Suite
//!
//! Validates:
//! 1. Syscall dispatch & argument passing (`SYS_GETPID`, `SYS_GETPPID`, `SYS_UPTIME`, `SYS_TEST_ECHO`)
//! 2. `SYS_WRITE` to stdout with user-space buffer
//! 3. Pointer security bounds & MMU validation (NULL, IVT, supervisor text, supervisor heap, unmapped, overflow)
//! 4. Cooperative scheduling syscalls (`SYS_YIELD`, `SYS_SLEEP`)
//! 5. Full Ring 3 User Mode execution via `int 0x80`, validating echo, write, memory fault rejection, and clean exit

use core::arch::global_asm;
use core::sync::atomic::{AtomicI32, Ordering};

use crate::arch::x86_64::pit;
use crate::memory::paging::PAGE_MANAGER;
use crate::syscall::dispatcher::dispatch_syscall;
use crate::syscall::error::SyscallError;
use crate::syscall::numbers::*;
use crate::user::test::user_mode_setjmp_and_enter;
use crate::user::{setup_user_space, USER_CODE_VIRT_ADDR, USER_STACK_TOP};
use crate::{klog_info, klog_ok};

/// Atomic container for recording user-space exit status from Test 5.
pub static RECORDED_USER_EXIT: AtomicI32 = AtomicI32::new(-999);

extern "C" {
    pub static phase12_user_code_start: u8;
    pub static phase12_user_code_end: u8;
}

/// Returns the slice of bytes representing the Phase 12 user-mode test binary.
pub fn phase12_user_payload_bytes() -> &'static [u8] {
    let start = core::ptr::addr_of!(phase12_user_code_start) as usize;
    let end = core::ptr::addr_of!(phase12_user_code_end) as usize;
    let len = end - start;
    // SAFETY: Symbol boundaries defined in global assembly below.
    unsafe { core::slice::from_raw_parts(start as *const u8, len) }
}

global_asm!(
    r#"
    .section .rodata
    .global phase12_user_code_start
    .global phase12_user_code_end
    .code64
phase12_user_code_start:
    // Verify Ring 3 CPL
    mov ax, cs
    and ax, 3
    cmp ax, 3
    jne 99f

    // Test 1: SYS_TEST_ECHO (10), arg = 21 -> rax must be 42
    mov rax, 10
    mov rdi, 21
    int 0x80
    cmp rax, 42
    jne 99f

    // Test 2: SYS_WRITE (2), fd = 1, ptr = phase12_user_msg, len = 40
    lea rsi, [rip + phase12_user_msg]
    mov rax, 2
    mov rdi, 1
    mov rdx, 40
    int 0x80
    cmp rax, 40
    jne 99f

    // Test 3: SYS_WRITE (2) with supervisor pointer 0x0010_0000 -> must return -14
    mov rax, 2
    mov rdi, 1
    mov rsi, 0x00100000
    mov rdx, 16
    int 0x80
    cmp rax, -14
    jne 99f

    // Test 4: SYS_GETPID (4) -> rax must be >= 0
    mov rax, 4
    int 0x80
    cmp rax, 0
    jl 99f

    // Test 5: SYS_EXIT (1), exit_code = 0 (Success)
    mov rax, 1
    xor rdi, rdi
    int 0x80

    // Halt fallback if not exited
1:  pause
    jmp 1b

99: // Failure: SYS_EXIT with exit_code = 42
    mov rax, 1
    mov rdi, 42
    int 0x80
2:  pause
    jmp 2b

phase12_user_msg:
    .ascii "[USER MODE] Milestone D Syscall Active!\n"
phase12_user_code_end:
"#
);

/// Executes the comprehensive Milestone D System Call test suite.
pub fn run_syscall_tests() {
    klog_info!("Starting Phase 12 / Milestone D System Call Verification Suite...");

    // Test 1: System call dispatch & argument passing
    let pid = dispatch_syscall(SYS_GETPID, 0, 0, 0, 0, 0);
    let ppid = dispatch_syscall(SYS_GETPPID, 0, 0, 0, 0, 0);
    let uptime = dispatch_syscall(SYS_UPTIME, 0, 0, 0, 0, 0);
    let echo = dispatch_syscall(SYS_TEST_ECHO, 42, 0, 0, 0, 0);
    let echo_zero = dispatch_syscall(SYS_TEST_ECHO, 0, 0, 0, 0, 0);
    let nosys = dispatch_syscall(999, 0, 0, 0, 0, 0);

    if echo != 84 || echo_zero != 0 {
        panic!("SYS_TEST_ECHO argument passing mismatch: got {}", echo);
    }
    if uptime < 10 {
        panic!("SYS_UPTIME integrity mismatch: got {}", uptime);
    }
    if pid < 0 || ppid < 0 {
        panic!("SYS_GETPID/SYS_GETPPID mismatch: pid={}, ppid={}", pid, ppid);
    }
    if nosys != SyscallError::NoSuchSyscall.as_i64() {
        panic!("Unimplemented syscall did not return ENOSYS: got {}", nosys);
    }
    klog_ok!(
        "Syscall Test 1 Passed: Core dispatch, arguments, and ENOSYS verified (PID={}, PPID={}, Uptime={} ticks)",
        pid,
        ppid,
        uptime
    );

    // Ensure user address space is mapped
    {
        let page_mgr_lock = PAGE_MANAGER.lock();
        let page_mgr = page_mgr_lock.as_ref().expect("PageManager not available");
        let mapped = page_mgr.get_page_flags(USER_CODE_VIRT_ADDR).is_some();
        drop(page_mgr_lock);
        if !mapped {
            setup_user_space().expect("Failed to initialize user address space");
        }
    }

    // Test 2: SYS_WRITE to stdout with user buffer
    let test_msg = b"[SYSCALL TEST] SYS_WRITE valid user buffer verified\n";
    let user_buf_addr = USER_CODE_VIRT_ADDR + 0x800;
    // Copy test message into user-accessible memory
    unsafe {
        core::ptr::copy_nonoverlapping(test_msg.as_ptr(), user_buf_addr as *mut u8, test_msg.len());
    }

    let write_res = dispatch_syscall(SYS_WRITE, 1, user_buf_addr, test_msg.len() as u64, 0, 0);
    if write_res != test_msg.len() as i64 {
        panic!("SYS_WRITE failed: expected {}, got {}", test_msg.len(), write_res);
    }
    let bad_fd = dispatch_syscall(SYS_WRITE, 99, user_buf_addr, test_msg.len() as u64, 0, 0);
    if bad_fd != SyscallError::BadFileDescriptor.as_i64() {
        panic!("SYS_WRITE with invalid fd did not return EBADF: got {}", bad_fd);
    }
    klog_ok!(
        "Syscall Test 2 Passed: SYS_WRITE to stdout with user buffer verified ({} bytes written)",
        write_res
    );

    // Test 3: Pointer security validation (Invalid pointers must be rejected with -14)
    let r_null = dispatch_syscall(SYS_WRITE, 1, 0x0, 16, 0, 0);
    if r_null != SyscallError::BadAddress.as_i64() {
        panic!("Null pointer was not rejected with EFAULT (-14): got {}", r_null);
    }

    let r_ivt = dispatch_syscall(SYS_WRITE, 1, 0x500, 16, 0, 0);
    if r_ivt != SyscallError::BadAddress.as_i64() {
        panic!("Low IVT pointer was not rejected with EFAULT (-14): got {}", r_ivt);
    }

    let r_kernel = dispatch_syscall(SYS_WRITE, 1, 0x0010_0000, 16, 0, 0);
    if r_kernel != SyscallError::BadAddress.as_i64() {
        panic!("Supervisor text pointer was not rejected with EFAULT (-14): got {}", r_kernel);
    }

    let r_heap = dispatch_syscall(SYS_WRITE, 1, 0xFFFF_9000_0000_0000, 16, 0, 0);
    if r_heap != SyscallError::BadAddress.as_i64() {
        panic!("Higher-half supervisor heap pointer was not rejected with EFAULT (-14): got {}", r_heap);
    }

    let r_unmapped = dispatch_syscall(SYS_WRITE, 1, 0x0000_0000_7000_0000, 16, 0, 0);
    if r_unmapped != SyscallError::BadAddress.as_i64() {
        panic!("Unmapped virtual address was not rejected with EFAULT (-14): got {}", r_unmapped);
    }

    let r_overflow = dispatch_syscall(SYS_WRITE, 1, 0xFFFF_FFFF_FFFF_FFF0, 32, 0, 0);
    if r_overflow != SyscallError::BadAddress.as_i64() {
        panic!("Overflowing pointer was not rejected with EFAULT (-14): got {}", r_overflow);
    }
    klog_ok!("Syscall Test 3 Passed: Pointer security bounds & MMU validation verified (EFAULT = -14)");

    // Test 4: Cooperative scheduling syscalls (SYS_YIELD & SYS_SLEEP)
    let yield_res = dispatch_syscall(SYS_YIELD, 0, 0, 0, 0, 0);
    if yield_res != 0 {
        panic!("SYS_YIELD failed: got {}", yield_res);
    }

    let sleep_start = pit::get_ticks();
    let sleep_res = dispatch_syscall(SYS_SLEEP, 5, 0, 0, 0, 0);
    let sleep_elapsed = pit::get_ticks() - sleep_start;
    if sleep_res != 0 || sleep_elapsed < 5 {
        panic!("SYS_SLEEP failed: res={}, elapsed={}", sleep_res, sleep_elapsed);
    }
    klog_ok!(
        "Syscall Test 4 Passed: SYS_YIELD and SYS_SLEEP verified (elapsed: {} ticks)",
        sleep_elapsed
    );

    // Test 5: Full Ring 3 User Mode execution via int 0x80
    klog_info!("Syscall Test 5: Executing full Ring 3 User Mode test via int 0x80...");
    RECORDED_USER_EXIT.store(-999, Ordering::Release);

    let payload = phase12_user_payload_bytes();
    let dest_ptr = USER_CODE_VIRT_ADDR as *mut u8;
    unsafe {
        core::ptr::copy_nonoverlapping(payload.as_ptr(), dest_ptr, payload.len());
    }

    unsafe {
        user_mode_setjmp_and_enter(USER_CODE_VIRT_ADDR, USER_STACK_TOP);
    }

    let exit_code = RECORDED_USER_EXIT.load(Ordering::Acquire);
    if exit_code != 0 {
        panic!("Syscall Test 5 Failed: User Mode execution exited with error code {}", exit_code);
    }
    klog_ok!("Syscall Test 5 Passed: Ring 3 int 0x80 roundtrip, echo, write, pointer rejection & clean exit verified");

    klog_ok!("MILESTONE D VERIFIED: System Call ABI & Pointer Security fully operational");
}
