//! Phase 13 — Scheduler & Preemptive Multitasking Verification Suite
//!
//! Validates:
//! 1. Priority run queue ordering & FIFO fairness
//! 2. Multi-task cooperative multitasking (`yield_now()`)
//! 3. Sleep queue and tick-based timed task suspension (`sleep_current()`)
//! 4. Preemptive multitasking driven by 100 Hz PIT timer interrupt
//! 5. Automatic task termination via `kernel_thread_exit_stub` and state inspection

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::arch::x86_64::pit;
use crate::process::manager::PROCESS_MANAGER;
use crate::process::pid::ProcessId;
use crate::process::state::ProcessState;
use crate::scheduler::priority::Priority;
use crate::scheduler::queue::RunQueue;
use crate::scheduler::{
    register_task, sleep_current, stats, yield_now,
};
use crate::{klog_info, klog_ok};

// --- Test 2: Cooperative Multitasking Statics ---
static COOP_STEP: AtomicU32 = AtomicU32::new(0);
static TASK_ALPHA_DONE: AtomicBool = AtomicBool::new(false);
static TASK_BETA_DONE: AtomicBool = AtomicBool::new(false);
static TASK_GAMMA_DONE: AtomicBool = AtomicBool::new(false);

extern "C" fn worker_alpha() {
    // Step 0 -> Step 1
    if COOP_STEP.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        yield_now();
    }
    // Step 3 -> Step 4
    if COOP_STEP.compare_exchange(3, 4, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        TASK_ALPHA_DONE.store(true, Ordering::Release);
        yield_now();
    }
}

extern "C" fn worker_beta() {
    // Step 1 -> Step 2
    if COOP_STEP.compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        yield_now();
    }
    // Step 4 -> Step 5
    if COOP_STEP.compare_exchange(4, 5, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        TASK_BETA_DONE.store(true, Ordering::Release);
        yield_now();
    }
}

extern "C" fn worker_gamma() {
    // Step 2 -> Step 3
    if COOP_STEP.compare_exchange(2, 3, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        yield_now();
    }
    // Step 5 -> Step 6
    if COOP_STEP.compare_exchange(5, 6, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        TASK_GAMMA_DONE.store(true, Ordering::Release);
        yield_now();
    }
}

// --- Test 3: Sleep Queue Statics ---
static SLEEPER_START_TICK: AtomicU64 = AtomicU64::new(0);
static SLEEPER_END_TICK: AtomicU64 = AtomicU64::new(0);
static SLEEPER_WOKEN: AtomicBool = AtomicBool::new(false);

extern "C" fn sleeper_worker() {
    let start = pit::get_ticks();
    SLEEPER_START_TICK.store(start, Ordering::Release);

    // Sleep for 5 ticks (50 ms @ 100 Hz)
    sleep_current(5);

    let end = pit::get_ticks();
    SLEEPER_END_TICK.store(end, Ordering::Release);
    SLEEPER_WOKEN.store(true, Ordering::Release);
}

// --- Test 4: Preemption Statics ---
static PREEMPT_WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static STOP_PREEMPT: AtomicBool = AtomicBool::new(false);
static PREEMPT_LOOP_COUNT: AtomicU64 = AtomicU64::new(0);

extern "C" fn preempt_worker() {
    PREEMPT_WORKER_STARTED.store(true, Ordering::Release);
    while !STOP_PREEMPT.load(Ordering::Acquire) {
        PREEMPT_LOOP_COUNT.fetch_add(1, Ordering::Relaxed);
        core::hint::spin_loop();
    }
}

// --- Test 5: Automatic Exit Statics ---
static AUTO_EXIT_RAN: AtomicBool = AtomicBool::new(false);

extern "C" fn auto_exit_worker() {
    AUTO_EXIT_RAN.store(true, Ordering::Release);
    // Returns normally; kernel_thread_exit_stub will intercept ret!
}

/// Executes the comprehensive Phase 13 verification suite.
pub fn run_scheduler_tests() {
    klog_info!("Starting Phase 13 Scheduler & Multitasking Verification Suite...");

    // Test 1: Priority Run Queue Order Verification
    {
        let mut rq = RunQueue::new();
        let p_idle = ProcessId(100);
        let p_low = ProcessId(101);
        let p_norm = ProcessId(102);
        let p_high = ProcessId(103);
        let p_rt = ProcessId(104);

        // Enqueue in arbitrary order
        rq.enqueue(p_idle, Priority::Idle);
        rq.enqueue(p_low, Priority::Low);
        rq.enqueue(p_norm, Priority::Normal);
        rq.enqueue(p_high, Priority::High);
        rq.enqueue(p_rt, Priority::Realtime);

        if rq.len() != 5 {
            panic!("RunQueue length mismatch: expected 5, got {}", rq.len());
        }

        // Verify strict priority dequeue ordering: RT -> High -> Normal -> Low -> Idle
        let d1 = rq.dequeue();
        let d2 = rq.dequeue();
        let d3 = rq.dequeue();
        let d4 = rq.dequeue();
        let d5 = rq.dequeue();

        if d1 != Some(p_rt)
            || d2 != Some(p_high)
            || d3 != Some(p_norm)
            || d4 != Some(p_low)
            || d5 != Some(p_idle)
        {
            panic!(
                "RunQueue priority ordering failure: [{:?}, {:?}, {:?}, {:?}, {:?}]",
                d1, d2, d3, d4, d5
            );
        }

        // Verify FIFO order within same priority bucket
        rq.enqueue(ProcessId(201), Priority::Normal);
        rq.enqueue(ProcessId(202), Priority::Normal);
        rq.enqueue(ProcessId(203), Priority::Normal);

        if rq.dequeue() != Some(ProcessId(201))
            || rq.dequeue() != Some(ProcessId(202))
            || rq.dequeue() != Some(ProcessId(203))
        {
            panic!("RunQueue FIFO order within bucket violated");
        }

        klog_ok!("Phase 13 Test 1 Passed: Run queue multi-level priority & FIFO ordering verified");
    }

    // Test 2: Multi-Task Cooperative Multitasking (yield_now)
    {
        COOP_STEP.store(0, Ordering::SeqCst);
        TASK_ALPHA_DONE.store(false, Ordering::Release);
        TASK_BETA_DONE.store(false, Ordering::Release);
        TASK_GAMMA_DONE.store(false, Ordering::Release);

        let mut pm = PROCESS_MANAGER.lock();
        let pid_a = pm.spawn_kernel("task_alpha", worker_alpha, 15);
        let pid_b = pm.spawn_kernel("task_beta", worker_beta, 15);
        let pid_c = pm.spawn_kernel("task_gamma", worker_gamma, 15);
        drop(pm);

        register_task(pid_a, Priority::Normal);
        register_task(pid_b, Priority::Normal);
        register_task(pid_c, Priority::Normal);

        // Drive cooperative schedule until all three tasks finish their interleaved steps
        let mut loop_limit = 200;
        while (!TASK_ALPHA_DONE.load(Ordering::Acquire)
            || !TASK_BETA_DONE.load(Ordering::Acquire)
            || !TASK_GAMMA_DONE.load(Ordering::Acquire))
            && loop_limit > 0
        {
            yield_now();
            loop_limit -= 1;
        }

        if COOP_STEP.load(Ordering::Acquire) != 6 {
            panic!(
                "Cooperative multitasking step sequence failure: expected 6, got {}",
                COOP_STEP.load(Ordering::Acquire)
            );
        }

        klog_ok!("Phase 13 Test 2 Passed: 3-task cooperative interleaving verified (6 sequential handovers)");
    }

    // Test 3: Sleep Queue & Timed Task Suspension
    {
        SLEEPER_WOKEN.store(false, Ordering::Release);

        let mut pm = PROCESS_MANAGER.lock();
        let pid_sleep = pm.spawn_kernel("task_sleeper", sleeper_worker, 10);
        drop(pm);

        register_task(pid_sleep, Priority::High);

        // Yield to allow sleeper task to start and enter sleep
        yield_now();

        // Spin/yield until timer ticks advance and scheduler wakes sleeper task
        let wait_start = pit::get_ticks();
        while !SLEEPER_WOKEN.load(Ordering::Acquire) && pit::get_ticks() < wait_start + 30 {
            yield_now();
            core::hint::spin_loop();
        }

        if !SLEEPER_WOKEN.load(Ordering::Acquire) {
            panic!("Sleeper task was not woken by timer interrupt");
        }

        let start = SLEEPER_START_TICK.load(Ordering::Acquire);
        let end = SLEEPER_END_TICK.load(Ordering::Acquire);
        let elapsed = end.saturating_sub(start);

        if elapsed < 5 {
            panic!("Sleep duration too short: elapsed {} ticks (expected >= 5)", elapsed);
        }

        klog_ok!(
            "Phase 13 Test 3 Passed: Timed task suspension verified (slept {} ticks, target >= 5)",
            elapsed
        );
    }

    // Test 4: Preemptive Multitasking via Timer Ticks
    {
        PREEMPT_WORKER_STARTED.store(false, Ordering::Release);
        STOP_PREEMPT.store(false, Ordering::Release);
        PREEMPT_LOOP_COUNT.store(0, Ordering::Release);

        let mut pm = PROCESS_MANAGER.lock();
        let pid_preempt = pm.spawn_kernel("task_preempt", preempt_worker, 15);
        drop(pm);

        register_task(pid_preempt, Priority::Normal);

        let (_, pre_preemptions, _) = stats();

        // Hand over execution to preempt_worker
        yield_now();

        // At this point, preempt_worker was running in a tight loop and was preempted by PIT timer ticks!
        let worker_started = PREEMPT_WORKER_STARTED.load(Ordering::Acquire);
        let loops = PREEMPT_LOOP_COUNT.load(Ordering::Acquire);
        let (_, post_preemptions, _) = stats();

        // Stop the worker
        STOP_PREEMPT.store(true, Ordering::Release);
        yield_now();

        if !worker_started || loops == 0 {
            panic!("Preempt worker did not execute loop cycles: loops = {}", loops);
        }

        klog_ok!(
            "Phase 13 Test 4 Passed: Hardware timer preemption verified (preemptions: pre={}, post={}, worker loops={})",
            pre_preemptions,
            post_preemptions,
            loops
        );
    }

    // Test 5: Automatic Task Termination via Exit Trampoline
    {
        AUTO_EXIT_RAN.store(false, Ordering::Release);

        let mut pm = PROCESS_MANAGER.lock();
        let pid_auto = pm.spawn_kernel("task_auto_exit", auto_exit_worker, 20);
        drop(pm);

        register_task(pid_auto, Priority::Normal);

        yield_now();

        if !AUTO_EXIT_RAN.load(Ordering::Acquire) {
            panic!("Auto exit worker did not run");
        }

        let pm = PROCESS_MANAGER.lock();
        let state = pm.get(pid_auto).map(|p| p.state);
        drop(pm);

        if state != Some(ProcessState::Terminated(0)) {
            panic!("Auto exit worker state mismatch: expected Terminated(0), got {:?}", state);
        }

        klog_ok!("Phase 13 Test 5 Passed: Kernel thread exit stub & lifecycle termination verified");
    }

    let (switches, preempts, yields) = stats();
    klog_ok!(
        "Scheduler Diagnostics: {} total switches, {} preemptions, {} cooperative yields",
        switches,
        preempts,
        yields
    );

    klog_ok!("PHASE 13 VERIFIED: Scheduler & Preemptive Multitasking fully operational");
}
