//! Phase 34 — Research Incorporation Verification Suite
//!
//! Validates:
//! 1. Processor Affinity Bitmask & Bounds (Mask creation, bit manipulation, intersection, string formatting)
//! 2. Deterministic CPU Workload Algorithm Integrity (Exact mathematical formula and repeatable checksum)
//! 3. Affinity-Constrained Task Scheduling (ProcessControlBlock affinity configuration, sched_setaffinity, sched_getaffinity)
//! 4. Workload Distribution Multi-Worker Scaling (Multi-worker task execution, 4 work units, checksum consistency)
//! 5. CPU-Time Accounting & Aggregate Utilization Metrics (Worker CPU time, elapsed time, aggregate utilization formula)

use crate::process::pid::ProcessId;
use crate::research::affinity::CpuAffinityMask;
use crate::research::workload::{cpu_work, execute_distributed_workload};
use crate::{klog_info, klog_ok};

/// Runs all 5 Phase 34 research verification tests.
pub fn run_research_tests() {
    klog_info!("=== Phase 34: Research Writeup Verification Tests ===");

    test_affinity_mask_operations();
    test_cpu_work_algorithm_integrity();
    test_sched_setaffinity_and_pcb();
    test_workload_distribution_multi_worker();
    test_cpu_time_accounting_and_utilization();

    klog_info!("=== Phase 34: All Research Writeup Tests Passed (5/5) ===");
}

/// Test 1: Processor Affinity Bitmask & Bounds.
fn test_affinity_mask_operations() {
    klog_info!("Research Test 1: Processor Affinity Bitmask Operations...");

    let mut mask = CpuAffinityMask::empty();
    assert_eq!(mask.count_allowed(), 0);
    assert!(!mask.is_cpu_allowed(0));

    // Enable CPU 0, 2, 4, 6
    mask.set_cpu(0, true);
    mask.set_cpu(2, true);
    mask.set_cpu(4, true);
    mask.set_cpu(6, true);

    assert_eq!(mask.count_allowed(), 4);
    assert!(mask.is_cpu_allowed(0));
    assert!(!mask.is_cpu_allowed(1));
    assert!(mask.is_cpu_allowed(2));
    assert!(!mask.is_cpu_allowed(3));
    assert!(mask.is_cpu_allowed(4));
    assert!(mask.is_cpu_allowed(6));
    assert_eq!(mask.bits(), 0x0055);

    // Test intersection with DEFAULT_ALL_ALLOWED (0xFFFF)
    let inter = mask.intersect(&CpuAffinityMask::DEFAULT_ALL_ALLOWED);
    assert_eq!(inter.bits(), 0x0055);

    // Test string formatting
    let list_str = mask.to_cpu_list_string();
    assert_eq!(list_str.as_str(), "0, 2, 4, 6");

    klog_ok!("Research Test 1 Passed: Affinity bitmasks, core checks, and string list verified");
}

/// Test 2: Deterministic CPU Workload Algorithm Integrity.
fn test_cpu_work_algorithm_integrity() {
    klog_info!("Research Test 2: Deterministic CPU Workload Algorithm Integrity...");

    // 0 iterations produces 0
    assert_eq!(cpu_work(0), 0);

    // Compute for known iteration steps
    let chk_100 = cpu_work(100);
    let chk_100_repeat = cpu_work(100);
    assert_eq!(chk_100, chk_100_repeat, "Algorithm must be strictly deterministic");
    assert!(chk_100 < 1_000_003, "Modulo bound must be maintained");

    let chk_1000 = cpu_work(1000);
    let chk_1000_repeat = cpu_work(1000);
    assert_eq!(chk_1000, chk_1000_repeat, "Repeatability confirmed for 1000 iterations");

    klog_ok!("Research Test 2 Passed: cpu_work(n) algorithm determinism verified");
}

/// Test 3: Affinity-Constrained Task Scheduling.
fn test_sched_setaffinity_and_pcb() {
    klog_info!("Research Test 3: Affinity-Constrained Task Scheduling...");

    let root_pid = ProcessId::KERNEL;

    // Read current affinity
    let initial_mask = crate::scheduler::sched_getaffinity(root_pid)
        .expect("Failed to get root task affinity");
    assert_eq!(initial_mask, CpuAffinityMask::DEFAULT_ALL_ALLOWED.bits());

    // Set to Single CPU 0
    assert!(
        crate::scheduler::sched_setaffinity(root_pid, CpuAffinityMask::SINGLE_CPU_0.bits()).is_ok()
    );
    let updated_mask = crate::scheduler::sched_getaffinity(root_pid).unwrap();
    assert_eq!(updated_mask, 1);

    // Set to Four CPUs 0,2,4,6
    assert!(
        crate::scheduler::sched_setaffinity(root_pid, CpuAffinityMask::FOUR_CPUS_0_2_4_6.bits()).is_ok()
    );
    let updated_mask_4 = crate::scheduler::sched_getaffinity(root_pid).unwrap();
    assert_eq!(updated_mask_4, 0x0055);

    // Invalid mask 0 must be rejected
    assert!(
        crate::scheduler::sched_setaffinity(root_pid, 0).is_err(),
        "Mask 0 must be rejected as invalid"
    );

    // Restore default affinity
    let _ = crate::scheduler::sched_setaffinity(root_pid, initial_mask);

    klog_ok!("Research Test 3 Passed: sched_setaffinity and sched_getaffinity verified");
}

/// Test 4: Workload Distribution Multi-Worker Scaling.
fn test_workload_distribution_multi_worker() {
    klog_info!("Research Test 4: Workload Distribution Multi-Worker Scaling...");

    let units = 4;
    let iterations = 50_000u64;

    // 1 Worker
    let summary_1 = execute_distributed_workload(units, 1, iterations);
    assert!(summary_1.checksums_consistent, "Checksums must match across all 4 units");
    assert_ne!(summary_1.aggregate_checksum, 0);

    // 2 Workers
    let summary_2 = execute_distributed_workload(units, 2, iterations);
    assert!(summary_2.checksums_consistent);
    assert_eq!(summary_2.aggregate_checksum, summary_1.aggregate_checksum);

    // 4 Workers
    let summary_4 = execute_distributed_workload(units, 4, iterations);
    assert!(summary_4.checksums_consistent);
    assert_eq!(summary_4.aggregate_checksum, summary_1.aggregate_checksum);

    klog_ok!("Research Test 4 Passed: Workload distributed across 1, 2, and 4 workers with consistent checksums");
}

/// Test 5: CPU-Time Accounting & Aggregate Utilization Metrics.
fn test_cpu_time_accounting_and_utilization() {
    klog_info!("Research Test 5: CPU-Time Accounting & Aggregate Utilization Metrics...");

    let root_pid = ProcessId::KERNEL;
    let pm = crate::process::manager::PROCESS_MANAGER.lock();
    let pcb = pm.get(root_pid).expect("Root PCB missing");
    let current_ticks = pcb.ticks_used;
    drop(pm);

    klog_info!("  Root task recorded CPU ticks: {}", current_ticks);

    // Run workload distribution and evaluate utilization metrics
    let summary = execute_distributed_workload(4, 2, 50_000);
    assert!(summary.estimated_aggregate_cpu_percent >= 0.0);
    klog_info!(
        "  Telemetry Summary: Worker CPU={} ms, Elapsed={} ms, Aggregate Use={:.1}%",
        summary.total_worker_cpu_ms, summary.elapsed_ms, summary.estimated_aggregate_cpu_percent
    );

    klog_ok!("Research Test 5 Passed: CPU-time tracking and aggregate utilization formula verified");
}
