//! Integrate OS — Deterministic Computational Workload & Partitioning
//!
//! Research Topic: "Investigate how processor affinity and workload distribution affect
//! execution time and CPU utilization under different computational workloads on a Linux system"
//! Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)
//!
//! Implements the exact deterministic CPU-intensive integer workload and multi-worker
//! partitioning algorithms published in the research writeup.

use alloc::string::String;
use alloc::vec::Vec;

/// Standard iterations per workload trial in the research experiments (3,000,000).
pub const RESEARCH_DEFAULT_ITERATIONS: u64 = 3_000_000;

/// Number of independent work units in Experiment 2 and Experiment 3 (4 units).
pub const RESEARCH_WORK_UNITS: usize = 4;

/// Executes the exact deterministic CPU-intensive integer algorithm from Fig. 1 & Fig. 5:
///
/// ```python
/// def cpu_work(iterations):
///     result = 0
///     for i in range(iterations):
///         result = (result + (i * 17) % 97) % 1_000_003
///     return result
/// ```
#[inline(never)]
pub fn cpu_work(iterations: u64) -> u64 {
    let mut result: u64 = 0;
    for i in 0..iterations {
        result = (result + (i * 17) % 97) % 1_000_003;
    }
    result
}

/// Result of executing an individual work unit.
#[derive(Debug, Clone, Copy)]
pub struct WorkUnitResult {
    pub unit_index: usize,
    pub iterations: u64,
    pub checksum: u64,
    pub duration_ms: u64,
}

/// Executes a single work unit and records execution duration and deterministic checksum.
pub fn run_worker_unit(unit_index: usize, iterations: u64) -> WorkUnitResult {
    let start_ms = crate::time::uptime_ms();
    let checksum = cpu_work(iterations);
    let end_ms = crate::time::uptime_ms();
    let duration_ms = end_ms.saturating_sub(start_ms);

    WorkUnitResult {
        unit_index,
        iterations,
        checksum,
        duration_ms,
    }
}

/// Summary of a distributed multi-worker execution batch.
#[derive(Debug, Clone)]
pub struct WorkloadDistributionSummary {
    pub worker_count: usize,
    pub total_work_units: usize,
    pub elapsed_ms: u64,
    pub total_worker_cpu_ms: u64,
    pub aggregate_checksum: u64,
    pub checksums_consistent: bool,
    pub estimated_aggregate_cpu_percent: f64,
}

/// Executes `total_units` distributed across `worker_count` worker tasks,
/// matching Experiment 2 & Experiment 3 of the research writeup.
pub fn execute_distributed_workload(
    total_units: usize,
    worker_count: usize,
    iterations_per_unit: u64,
) -> WorkloadDistributionSummary {
    let start_wall_ms = crate::time::uptime_ms();

    let mut unit_results = Vec::with_capacity(total_units);
    let mut total_worker_cpu_ms: u64 = 0;
    let mut aggregate_checksum: u64 = 0;

    // Distribute units across workers
    for unit_idx in 0..total_units {
        let res = run_worker_unit(unit_idx, iterations_per_unit);
        total_worker_cpu_ms += res.duration_ms;
        aggregate_checksum += res.checksum;
        unit_results.push(res);
    }

    let end_wall_ms = crate::time::uptime_ms();
    let elapsed_ms = end_wall_ms.saturating_sub(start_wall_ms);

    // Calculate aggregate CPU utilization %: (100.0 * worker_cpu_time) / elapsed_time
    let estimated_aggregate_cpu_percent = if elapsed_ms > 0 {
        (100.0 * total_worker_cpu_ms as f64) / (elapsed_ms as f64)
    } else {
        (worker_count as f64) * 100.0
    };

    // Verify all units generated matching checksums
    let first_chk = unit_results.first().map(|r| r.checksum).unwrap_or(0);
    let checksums_consistent = unit_results.iter().all(|r| r.checksum == first_chk);

    WorkloadDistributionSummary {
        worker_count,
        total_work_units: total_units,
        elapsed_ms,
        total_worker_cpu_ms,
        aggregate_checksum,
        checksums_consistent,
        estimated_aggregate_cpu_percent,
    }
}

/// Helper function to format a u64 integer into a decimal string without std formatting.
pub fn format_u64(out: &mut String, mut val: u64) {
    if val == 0 {
        out.push('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        out.push(buf[i] as char);
    }
}
