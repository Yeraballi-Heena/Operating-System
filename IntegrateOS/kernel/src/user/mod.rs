//! Integrate OS — User Mode (Ring 3 Privilege Separation)
//!
//! Provides:
//! - User space memory layout and page table isolation (`USER_ACCESSIBLE`)
//! - User mode entry trampoline via `iretq`
//! - Hardware-enforced Ring 3 privilege isolation verification (#GP on `cli`, #PF on supervisor memory)
//! - Software system call dispatcher via `int 0x80` (Vector 128)
//! - Clean execution return continuation to kernel mode

pub mod test;
pub mod trampoline;

use core::sync::atomic::Ordering;

use crate::arch::x86_64::exceptions::{USER_GP_CAUGHT, USER_PF_CAUGHT, USER_TEST_ACTIVE};
use crate::arch::x86_64::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
use crate::arch::x86_64::state::CpuState;
use crate::memory::frame_allocator::FRAME_ALLOCATOR;
use crate::memory::paging::{PageTableFlags, PAGE_MANAGER};
use crate::user::test::{
    user_payload_bytes, USER_CALCULATION_RESULT, USER_SYSCALL_1_RECEIVED, USER_SYSCALL_2_RECEIVED,
};
use crate::{klog_info, klog_ok};

/// Canonical lower-half user code virtual address (1 GiB).
pub const USER_CODE_VIRT_ADDR: u64 = 0x0000_0000_4000_0000;

/// Canonical lower-half user stack virtual address (2 GiB).
pub const USER_STACK_VIRT_ADDR: u64 = 0x0000_0000_8000_0000;

/// Top of the 4 KiB user stack.
pub const USER_STACK_TOP: u64 = USER_STACK_VIRT_ADDR + 4096;

/// Prepares user-space page mappings and copies the test payload.
pub fn setup_user_space() -> Result<(u64, u64), &'static str> {
    let mut page_mgr_lock = PAGE_MANAGER.lock();
    let page_mgr = page_mgr_lock.as_mut().ok_or("PageManager not available")?;

    // 1. Allocate dedicated physical frames for user code and user stack
    let code_frame = FRAME_ALLOCATOR
        .lock()
        .allocate_frame()
        .ok_or("Out of memory for user code frame")?;
    let stack_frame = FRAME_ALLOCATOR
        .lock()
        .allocate_frame()
        .ok_or("Out of memory for user stack frame")?;

    let user_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE;

    // 2. Map user virtual addresses into active 4-level page tables
    page_mgr.map_page(USER_CODE_VIRT_ADDR, code_frame, user_flags)?;
    page_mgr.map_page(USER_STACK_VIRT_ADDR, stack_frame, user_flags)?;

    // 3. Copy user binary payload into the user code page
    let payload = user_payload_bytes();
    let dest_ptr = USER_CODE_VIRT_ADDR as *mut u8;
    // SAFETY: USER_CODE_VIRT_ADDR was just identity-backed and mapped writable.
    unsafe {
        core::ptr::copy_nonoverlapping(payload.as_ptr(), dest_ptr, payload.len());
    }

    Ok((USER_CODE_VIRT_ADDR, USER_STACK_TOP))
}

/// Software interrupt / System Call handler for Vector 128 (`int 0x80`).
pub fn handle_user_syscall(state: &mut CpuState) {
    let syscall_num = state.rdi;
    let arg1 = state.rsi;
    let cpl = (state.cs & 3) as u8;

    // During Phase 10 verification test, route to dedicated test handler
    if USER_TEST_ACTIVE.load(Ordering::Acquire) {
        match syscall_num {
            1 => {
                // SYS_USER_LOG: Record arithmetic calculation payload from User Mode
                USER_SYSCALL_1_RECEIVED.store(true, Ordering::Release);
                USER_CALCULATION_RESULT.store(arg1, Ordering::Release);
                klog_ok!(
                    "User Syscall 1 (SYS_USER_LOG): payload={} (Origin: Ring {})",
                    arg1,
                    cpl
                );
                state.rax = 0; // Return success
            }
            2 => {
                // SYS_USER_EXIT: User program requested termination
                USER_SYSCALL_2_RECEIVED.store(true, Ordering::Release);
                klog_ok!(
                    "User Syscall 2 (SYS_USER_EXIT): exit_code={} (Origin: Ring {})",
                    arg1,
                    cpl
                );

                // Reconfigure iretq stack frame to return directly to kernel continuation
                // SAFETY: KERNEL_SAVED_RSP was captured before entering user mode.
                let saved_rsp = unsafe { test::KERNEL_SAVED_RSP };
                if saved_rsp != 0 {
                    state.cs = KERNEL_CODE_SELECTOR.0 as u64;
                    state.ss = KERNEL_DATA_SELECTOR.0 as u64;
                    state.rip = test::user_mode_return_point as *const () as u64;
                    state.rsp = saved_rsp;
                    state.rflags = 0x202; // IF=1, IOPL=0
                }
            }
            _ => {
                state.rax = 0xFFFF_FFFF_FFFF_FFFF;
            }
        }
        return;
    }

    // General System Call ABI Dispatch (Phase 12+ / Milestone D)
    let (num, a1, a2, a3, a4, a5) = if state.rax != 0 {
        (state.rax, state.rdi, state.rsi, state.rdx, state.r10, state.r8)
    } else {
        (state.rdi, state.rsi, state.rdx, state.rcx, state.r8, state.r9)
    };

    let ret = crate::syscall::dispatcher::dispatch_syscall(num, a1, a2, a3, a4, a5);
    state.rax = ret as u64;

    if num == crate::syscall::numbers::SYS_EXIT {
        crate::syscall::tests::RECORDED_USER_EXIT.store(a1 as i32, Ordering::Release);
        let saved_rsp = unsafe { test::KERNEL_SAVED_RSP };
        if saved_rsp != 0 {
            state.cs = KERNEL_CODE_SELECTOR.0 as u64;
            state.ss = KERNEL_DATA_SELECTOR.0 as u64;
            state.rip = test::user_mode_return_point as *const () as u64;
            state.rsp = saved_rsp;
            state.rflags = 0x202; // IF=1, IOPL=0
        }
    }
}

/// Executes the comprehensive User Mode verification suite.
pub fn run_user_mode_tests() {
    klog_info!("Starting Phase 10 User Mode Verification Suite...");

    // 1. Prepare user address space (code page @ 1 GiB, stack page @ 2 GiB)
    let (user_rip, user_rsp) = setup_user_space().expect("Failed to setup user address space");
    klog_ok!(
        "User Address Space Configured: Code={:#010x}, StackTop={:#010x} (USER_ACCESSIBLE)",
        user_rip,
        user_rsp
    );

    // 2. Enable user fault diagnostics
    USER_TEST_ACTIVE.store(true, Ordering::Release);

    // 3. Transition to User Mode (Ring 3) via setjmp continuation trampoline
    klog_info!("Executing Ring 3 Privilege Transition via iretq...");
    unsafe {
        test::user_mode_setjmp_and_enter(user_rip, user_rsp);
    }

    // 4. Execution resumes here after user program calls SYS_USER_EXIT and iretq returns
    USER_TEST_ACTIVE.store(false, Ordering::Release);

    // 5. Verify all architectural assertions
    let res = USER_CALCULATION_RESULT.load(Ordering::Acquire);
    if res != 427 {
        panic!("User Mode calculation mismatch: expected 427, got {}", res);
    }
    klog_ok!("User Mode Test 1 Passed: Ring 3 arithmetic computation verified (427)");

    if !USER_SYSCALL_1_RECEIVED.load(Ordering::Acquire) {
        panic!("User Mode Test 2 Failed: SYS_USER_LOG not received");
    }
    klog_ok!("User Mode Test 2 Passed: Software interrupt (int 0x80) system call roundtrip verified");

    let gp_caught = USER_GP_CAUGHT.load(Ordering::Acquire);
    if gp_caught == 0 {
        panic!("User Mode Test 3 Failed: Privileged instruction `cli` did not trigger #GP");
    }
    klog_ok!(
        "User Mode Test 3 Passed: Privileged instruction violation intercepted (#GP count = {})",
        gp_caught
    );

    let pf_caught = USER_PF_CAUGHT.load(Ordering::Acquire);
    if pf_caught == 0 {
        panic!("User Mode Test 4 Failed: Supervisor memory access did not trigger #PF");
    }
    klog_ok!(
        "User Mode Test 4 Passed: Supervisor memory protection intercepted (#PF count = {})",
        pf_caught
    );

    if !USER_SYSCALL_2_RECEIVED.load(Ordering::Acquire) {
        panic!("User Mode Test 5 Failed: SYS_USER_EXIT not received");
    }
    klog_ok!("User Mode Test 5 Passed: User space exit & kernel continuation verified");
    klog_ok!("PHASE 10 VERIFIED: User Mode (Ring 3 Privilege Separation) fully operational");
}
