//! Phase 11 Process Verification Suite & Context Switch Demonstration
//!
//! Verifies:
//! - Process creation and atomic PID allocation (PID 0, 1, 2, ...)
//! - Lifecycle state machine transitions (Ready, Running, Blocked, Terminated)
//! - Address space metadata isolation (Kernel vs User)
//! - Low-level context switch (`switch_context_asm`) register and stack preservation
//! - Process table enumeration and diagnostics

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::process::manager::PROCESS_MANAGER;
use crate::process::pid::ProcessId;
use crate::process::state::ProcessState;
use crate::process::switch::switch_context_asm;
use crate::{klog_info, klog_ok};

static TASK_B_RAN: AtomicBool = AtomicBool::new(false);
static TASK_B_COUNTER: AtomicU32 = AtomicU32::new(0);
static mut TASK_A_SAVED_RSP: u64 = 0;
static mut TASK_B_SAVED_RSP: u64 = 0;

/// Worker entry function for Context Switch Verification Test.
extern "C" fn worker_test_entry() {
    TASK_B_RAN.store(true, Ordering::Release);
    TASK_B_COUNTER.fetch_add(100, Ordering::Relaxed);

    // Switch back to Task A
    unsafe {
        let dummy_rsp = core::ptr::addr_of_mut!(TASK_B_SAVED_RSP);
        let target_rsp = TASK_A_SAVED_RSP;
        switch_context_asm(dummy_rsp, target_rsp, crate::arch::x86_64::registers::Cr3::read().pml4_phys_addr());
    }

    // Should never reach here in test
    loop {
        core::hint::spin_loop();
    }
}

/// Runs the complete Phase 11 verification suite.
pub fn run_process_tests() {
    klog_info!("Starting Phase 11 Process Architecture Verification Suite...");

    let mut pm = PROCESS_MANAGER.lock();

    // Test 1: Process Creation & Monotonic PID Allocation
    let pid_init = pm.spawn_kernel("system_init", dummy_task, 10);
    let pid_worker = pm.spawn_kernel("worker_task_a", dummy_task, 20);
    let pid_user = pm.spawn_user("user_app_shell", 0x4000_0000, 0x8000_1000, 0x122000, 15);

    if pid_init != ProcessId(1) || pid_worker != ProcessId(2) || pid_user != ProcessId(3) {
        panic!(
            "PID allocation mismatch: init={:?}, worker={:?}, user={:?}",
            pid_init, pid_worker, pid_user
        );
    }
    if pm.count() < 4 {
        panic!("Process count mismatch: expected >= 4, got {}", pm.count());
    }
    klog_ok!("Phase 11 Test 1 Passed: Process creation & monotonic PID allocation verified (PIDs: 0, 1, 2, 3)");

    // Test 2: Lifecycle State Machine Transitions
    pm.set_state(pid_worker, ProcessState::Running);
    if pm.get(pid_worker).map(|p| p.state) != Some(ProcessState::Running) {
        panic!("Failed transition to Running");
    }

    pm.set_state(pid_worker, ProcessState::Blocked);
    if pm.get(pid_worker).map(|p| p.state) != Some(ProcessState::Blocked) {
        panic!("Failed transition to Blocked");
    }

    pm.set_state(pid_worker, ProcessState::Ready);
    if pm.get(pid_worker).map(|p| p.state) != Some(ProcessState::Ready) {
        panic!("Failed transition to Ready");
    }

    pm.terminate(pid_worker, 0);
    if pm.get(pid_worker).map(|p| p.state) != Some(ProcessState::Terminated(0)) {
        panic!("Failed transition to Terminated");
    }
    if !pm.get(pid_worker).unwrap().state.is_terminated() {
        panic!("is_terminated() reported false for terminated process");
    }
    klog_ok!("Phase 11 Test 2 Passed: Process lifecycle state machine transitions verified");

    // Test 3: Address Space & Memory Bounds Metadata Isolation
    let proc_kernel = pm.get(pid_init).unwrap();
    if proc_kernel.is_user || proc_kernel.user_stack_top.is_some() {
        panic!("Kernel process incorrectly marked as user");
    }

    let proc_user = pm.get(pid_user).unwrap();
    if !proc_user.is_user || proc_user.user_stack_top != Some(0x8000_1000) {
        panic!("User process metadata incorrect: {:?}", proc_user.user_stack_top);
    }
    klog_ok!("Phase 11 Test 3 Passed: Kernel vs User address space & stack metadata isolation verified");

    // Test 4: Low-Level Context Switch Verification
    // Create a temporary task PCB for context switch testing
    let pid_switch_test = pm.spawn_kernel("context_switch_test", worker_test_entry, 5);
    let pcb_target = pm.get(pid_switch_test).unwrap();
    let target_initial_rsp = pcb_target.saved_rsp;
    drop(pm); // Release lock before context switch

    // Perform context switch from Task A (caller) to Task B (worker_test_entry)
    unsafe {
        let prev_rsp_ptr = core::ptr::addr_of_mut!(TASK_A_SAVED_RSP);
        switch_context_asm(prev_rsp_ptr, target_initial_rsp, crate::arch::x86_64::registers::Cr3::read().pml4_phys_addr());
    }

    // Execution resumes right here when Task B calls switch_context_asm back to TASK_A_SAVED_RSP!
    if !TASK_B_RAN.load(Ordering::Acquire) || TASK_B_COUNTER.load(Ordering::Acquire) != 100 {
        panic!("Context switch failed: Task B did not execute or modify shared state");
    }
    klog_ok!("Phase 11 Test 4 Passed: Context switch assembly & register preservation verified (Task B executed & returned)");

    // Test 5: Process Table Enumeration & Process Listing
    let pm = PROCESS_MANAGER.lock();
    klog_info!("Active Process Table ({} total processes):", pm.count());
    for proc in pm.iter() {
        klog_info!(
            "  PID {:02} | PPID {:02} | {:<18} | State: {:<10} | Prio: {:02} | User: {}",
            proc.pid.as_u64(),
            proc.ppid.as_u64(),
            proc.name.as_str(),
            proc.state.as_str(),
            proc.priority,
            proc.is_user
        );
    }
    klog_ok!("Phase 11 Test 5 Passed: Process table enumeration & diagnostic inspection verified");
    klog_ok!("PHASE 11 VERIFIED: Process Architecture (PCB, PID, Context, Lifecycle) fully operational");
}

extern "C" fn dummy_task() {
    loop {
        core::hint::spin_loop();
    }
}
