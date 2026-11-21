//! Integrate OS — Research Experiment Telemetry & Reporting
//!
//! Research Topic: "Investigate how processor affinity and workload distribution affect
//! execution time and CPU utilization under different computational workloads on a Linux system"
//! Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)
//!
//! Formats empirical experimental results matching the tables and figures in the research writeup.

use alloc::string::String;
use alloc::vec::Vec;

/// Formatted record for Experiment 1: Processor Affinity (Table 1).
#[derive(Debug, Clone)]
pub struct AffinityExperimentRecord {
    pub config_name: &'static str,
    pub cpus_label: &'static str,
    pub elapsed_ms: u64,
    pub checksum: u64,
}

/// Formatted record for Experiment 2: Workload Distribution (Table 2).
#[derive(Debug, Clone)]
pub struct DistributionExperimentRecord {
    pub worker_count: usize,
    pub elapsed_ms: u64,
    pub checksum_consistent: bool,
}

/// Formatted record for Experiment 3: CPU-Time Monitoring & Utilization (Table 3).
#[derive(Debug, Clone)]
pub struct CpuUtilizationRecord {
    pub worker_count: usize,
    pub elapsed_ms: u64,
    pub worker_cpu_ms: u64,
    pub estimated_aggregate_cpu_percent: f64,
}

/// Consolidated research results container.
#[derive(Debug, Clone)]
pub struct ResearchTelemetryReport {
    pub affinity_records: Vec<AffinityExperimentRecord>,
    pub distribution_records: Vec<DistributionExperimentRecord>,
    pub utilization_records: Vec<CpuUtilizationRecord>,
}

impl ResearchTelemetryReport {
    pub fn new() -> Self {
        Self {
            affinity_records: Vec::new(),
            distribution_records: Vec::new(),
            utilization_records: Vec::new(),
        }
    }

    /// Generates multi-line formatted summary output suitable for Terminal and Serial display.
    pub fn format_full_report(&self) -> String {
        let mut out = String::new();

        out.push_str("================================================================================\n");
        out.push_str("   OS RESEARCH: PROCESSOR AFFINITY & WORKLOAD DISTRIBUTION IN MULTICORE SYSTEMS\n");
        out.push_str("   Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)\n");
        out.push_str("================================================================================\n\n");

        out.push_str("--- Experiment 1: Effect of Processor Affinity (Table 1) ---\n");
        out.push_str("Configuration              CPUs Allowed          Elapsed (ms)   Checksum\n");
        out.push_str("-------------------------  --------------------  -------------  ----------\n");
        for rec in &self.affinity_records {
            out.push_str(rec.config_name);
            let pad = 27usize.saturating_sub(rec.config_name.len());
            for _ in 0..pad { out.push(' '); }

            out.push_str(rec.cpus_label);
            let pad2 = 22usize.saturating_sub(rec.cpus_label.len());
            for _ in 0..pad2 { out.push(' '); }

            let mut el_s = String::new();
            crate::research::workload::format_u64(&mut el_s, rec.elapsed_ms);
            el_s.push_str(" ms");
            out.push_str(&el_s);
            let pad3 = 15usize.saturating_sub(el_s.len());
            for _ in 0..pad3 { out.push(' '); }

            let mut chk_s = String::new();
            crate::research::workload::format_u64(&mut chk_s, rec.checksum);
            out.push_str(&chk_s);
            out.push('\n');
        }

        out.push_str("\n--- Experiment 2: Effect of Workload Distribution (Table 2) ---\n");
        out.push_str("Worker Processes   Total Work Units   Elapsed (ms)   Speedup vs 1 Worker\n");
        out.push_str("-----------------  -----------------  -------------  -------------------\n");
        let base_time = self.distribution_records.first().map(|r| r.elapsed_ms).unwrap_or(1).max(1);
        for rec in &self.distribution_records {
            let mut w_s = String::new();
            crate::research::workload::format_u64(&mut w_s, rec.worker_count as u64);
            out.push_str(&w_s);
            let pad = 19usize.saturating_sub(w_s.len());
            for _ in 0..pad { out.push(' '); }

            out.push_str("4 units            ");

            let mut el_s = String::new();
            crate::research::workload::format_u64(&mut el_s, rec.elapsed_ms);
            el_s.push_str(" ms");
            out.push_str(&el_s);
            let pad2 = 15usize.saturating_sub(el_s.len());
            for _ in 0..pad2 { out.push(' '); }

            let speedup_pct = if base_time >= rec.elapsed_ms {
                (100.0 * (base_time - rec.elapsed_ms) as f64) / (base_time as f64)
            } else {
                0.0
            };
            let mut sp_s = String::new();
            crate::research::workload::format_u64(&mut sp_s, speedup_pct as u64);
            sp_s.push_str("% reduction");
            out.push_str(&sp_s);
            out.push('\n');
        }

        out.push_str("\n--- Experiment 3: CPU-Time & Aggregate Utilization (Table 3) ---\n");
        out.push_str("Worker Processes   Elapsed (ms)   Worker CPU (ms)   Mean Aggregate CPU Use (%)\n");
        out.push_str("-----------------  -------------  ----------------  --------------------------\n");
        for rec in &self.utilization_records {
            let mut w_s = String::new();
            crate::research::workload::format_u64(&mut w_s, rec.worker_count as u64);
            out.push_str(&w_s);
            let pad = 19usize.saturating_sub(w_s.len());
            for _ in 0..pad { out.push(' '); }

            let mut el_s = String::new();
            crate::research::workload::format_u64(&mut el_s, rec.elapsed_ms);
            el_s.push_str(" ms");
            out.push_str(&el_s);
            let pad2 = 15usize.saturating_sub(el_s.len());
            for _ in 0..pad2 { out.push(' '); }

            let mut cpu_s = String::new();
            crate::research::workload::format_u64(&mut cpu_s, rec.worker_cpu_ms);
            cpu_s.push_str(" ms");
            out.push_str(&cpu_s);
            let pad3 = 18usize.saturating_sub(cpu_s.len());
            for _ in 0..pad3 { out.push(' '); }

            let mut pct_s = String::new();
            crate::research::workload::format_u64(&mut pct_s, rec.estimated_aggregate_cpu_percent as u64);
            pct_s.push('%');
            out.push_str(&pct_s);
            out.push('\n');
        }

        out.push_str("================================================================================\n");
        out
    }
}
