//! Integrate OS — Research Experiment Suite Runner
//!
//! Research Topic: "Investigate how processor affinity and workload distribution affect
//! execution time and CPU utilization under different computational workloads on a Linux system"
//! Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)

use crate::research::affinity::CpuAffinityMask;
use crate::research::telemetry::{
    AffinityExperimentRecord, CpuUtilizationRecord, DistributionExperimentRecord,
    ResearchTelemetryReport,
};
use crate::research::workload::{
    cpu_work, execute_distributed_workload, RESEARCH_WORK_UNITS,
};
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

/// Global cache of the most recent research benchmark execution results.
pub static RESEARCH_RESULTS: IrqSpinlock<Option<ResearchTelemetryReport>> = IrqSpinlock::new(None);

/// Executes all three experiments from the research paper and populates the global cache.
pub fn execute_all_research_experiments() -> ResearchTelemetryReport {
    klog_info!("[RESEARCH] Running Experiment 1: Effect of Processor Affinity (Table 1)...");

    let iterations = 200_000u64; // Scaled for deterministic bare-metal boot evaluation
    let mut report = ResearchTelemetryReport::new();

    // Experiment 1: Processor Affinity (Default vs Single CPU 0 vs Four CPUs 0,2,4,6)
    let configs = [
        ("Default_all_allowed", "0..15", CpuAffinityMask::DEFAULT_ALL_ALLOWED),
        ("Single_CPU_0", "[0]", CpuAffinityMask::SINGLE_CPU_0),
        ("Four_CPUs_0_2_4_6", "[0, 2, 4, 6]", CpuAffinityMask::FOUR_CPUS_0_2_4_6),
    ];

    for (name, label, mask) in configs {
        let start = crate::time::uptime_ms();
        let checksum = cpu_work(iterations);
        let elapsed = crate::time::uptime_ms().saturating_sub(start);

        klog_info!(
            "  Config '{:<20}' on CPUs {:<12}: elapsed={} ms, checksum={}",
            name, label, elapsed, checksum
        );

        report.affinity_records.push(AffinityExperimentRecord {
            config_name: name,
            cpus_label: label,
            elapsed_ms: elapsed,
            checksum,
        });

        // Set affinity on current running task
        let curr_pid = crate::scheduler::current_pid();
        let _ = crate::scheduler::sched_setaffinity(curr_pid, mask.bits());
    }

    // Restore default affinity for current running task
    let curr_pid = crate::scheduler::current_pid();
    let _ = crate::scheduler::sched_setaffinity(curr_pid, CpuAffinityMask::DEFAULT_ALL_ALLOWED.bits());

    klog_info!("[RESEARCH] Running Experiment 2: Effect of Workload Distribution (Table 2)...");
    let worker_counts = [1, 2, 4];
    for &workers in &worker_counts {
        let summary = execute_distributed_workload(RESEARCH_WORK_UNITS, workers, iterations / 2);
        klog_info!(
            "  Workers: {:<2} | Total Units: 4 | Elapsed: {} ms | Checksums Consistent: {}",
            workers, summary.elapsed_ms, summary.checksums_consistent
        );

        report.distribution_records.push(DistributionExperimentRecord {
            worker_count: workers,
            elapsed_ms: summary.elapsed_ms,
            checksum_consistent: summary.checksums_consistent,
        });
    }

    klog_info!("[RESEARCH] Running Experiment 3: CPU-Time & Aggregate Utilization (Table 3)...");
    for &workers in &worker_counts {
        let summary = execute_distributed_workload(RESEARCH_WORK_UNITS, workers, iterations / 2);
        klog_info!(
            "  Workers: {:<2} | Elapsed: {} ms | Worker CPU: {} ms | Aggregate Use: {:.1}%",
            workers, summary.elapsed_ms, summary.total_worker_cpu_ms, summary.estimated_aggregate_cpu_percent
        );

        report.utilization_records.push(CpuUtilizationRecord {
            worker_count: workers,
            elapsed_ms: summary.elapsed_ms,
            worker_cpu_ms: summary.total_worker_cpu_ms,
            estimated_aggregate_cpu_percent: summary.estimated_aggregate_cpu_percent,
        });
    }

    *RESEARCH_RESULTS.lock() = Some(report.clone());
    klog_ok!("[RESEARCH] All 3 Research Writeup Experiments Executed & Telemetry Cached");
    report
}
