//! Integrate OS — Multicore Research Subsystem (Phase 34)
//!
//! Research Topic: "Investigate how processor affinity and workload distribution affect
//! execution time and CPU utilization under different computational workloads on a Linux system"
//! Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201: Operating Systems)
//!
//! Provides:
//! - Hardware CPU affinity bitmasks and core pinning (`affinity.rs`)
//! - Deterministic computational integer workload and multi-worker partitioning (`workload.rs`)
//! - Worker CPU-time and aggregate utilization telemetry (`telemetry.rs`)
//! - In-kernel benchmark executor replicating research paper experiments (`benchmark.rs`)
//! - Verification test suite (`tests.rs`)

pub mod affinity;
pub mod benchmark;
pub mod telemetry;
pub mod tests;
pub mod workload;

use core::sync::atomic::{AtomicBool, Ordering};

pub use affinity::CpuAffinityMask;
pub use benchmark::{execute_all_research_experiments, RESEARCH_RESULTS};
pub use telemetry::{AffinityExperimentRecord, CpuUtilizationRecord, DistributionExperimentRecord, ResearchTelemetryReport};
pub use workload::{cpu_work, execute_distributed_workload, RESEARCH_DEFAULT_ITERATIONS, RESEARCH_WORK_UNITS};

use crate::{klog_info, klog_ok};

static RESEARCH_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns true if the research subsystem has been initialized.
#[inline(always)]
pub fn is_research_initialized() -> bool {
    RESEARCH_INITIALIZED.load(Ordering::Acquire)
}

/// Initializes the research subsystem, logs author details, and executes benchmark experiments.
pub fn init() {
    klog_info!("================================================================================");
    klog_info!("  RESEARCH INCORPORATION: PROCESSOR AFFINITY & WORKLOAD DISTRIBUTION IN OS");
    klog_info!("  Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)");
    klog_info!("================================================================================");

    // Execute research experiments and populate in-memory telemetry
    let _ = execute_all_research_experiments();

    RESEARCH_INITIALIZED.store(true, Ordering::Release);
    klog_ok!("Research Subsystem Initialized: Affinity masks, workload distributor, and CPU telemetry active");
}
