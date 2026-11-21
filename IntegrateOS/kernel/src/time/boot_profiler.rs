//! Integrate OS — Boot-Time Profiler & Micro OS Component Performance Tracking
//!
//! Tracks granular boot duration milestones from kernel entry to userspace,
//! and records boot time measurements after including each Micro OS component.

use alloc::vec::Vec;
use crate::sync::IrqSpinlock;
use crate::klog_info;

/// Represents an individual boot stage timestamp.
#[derive(Debug, Clone)]
pub struct BootMilestone {
    pub name: &'static str,
    pub timestamp_ms: u64,
    pub delta_ms: u64,
}

/// Represents a measured boot-time checkpoint for a specific OS component.
#[derive(Debug, Clone)]
pub struct ComponentBootMeasurement {
    pub component_name: &'static str,
    pub phase_number: u8,
    pub boot_time_ms: u64,
    pub notes: &'static str,
}

/// Central Boot-Time Profiler singleton state.
pub struct BootProfiler {
    milestones: Vec<BootMilestone>,
    component_measurements: Vec<ComponentBootMeasurement>,
    initial_boot_ms: u64,
}

impl BootProfiler {
    pub const fn new() -> Self {
        Self {
            milestones: Vec::new(),
            component_measurements: Vec::new(),
            initial_boot_ms: 0,
        }
    }

    /// Records a kernel boot milestone with monotonic timestamp.
    pub fn record_stage(&mut self, name: &'static str) {
        let now = crate::time::uptime_ms();
        let last = self.milestones.last().map(|m| m.timestamp_ms).unwrap_or(0);
        let delta_ms = now.saturating_sub(last);

        self.milestones.push(BootMilestone {
            name,
            timestamp_ms: now,
            delta_ms,
        });
    }

    /// Records a formal Micro OS component boot time measurement.
    pub fn record_component_boot_time(
        &mut self,
        component_name: &'static str,
        phase_number: u8,
        boot_time_ms: u64,
        notes: &'static str,
    ) {
        self.component_measurements.push(ComponentBootMeasurement {
            component_name,
            phase_number,
            boot_time_ms,
            notes,
        });

        klog_info!(
            "[BOOT-PROFILE] Checkpoint #{}: '{}' (Phase {}) -> Boot Time: {} ms ({})",
            self.component_measurements.len(),
            component_name,
            phase_number,
            boot_time_ms,
            notes
        );
    }

    /// Returns all recorded component boot measurements.
    pub fn component_measurements(&self) -> &[ComponentBootMeasurement] {
        &self.component_measurements
    }

    /// Returns initial boot timestamp.
    pub fn initial_boot_ms(&self) -> u64 {
        self.initial_boot_ms
    }

    /// Emits a structured comparison table of boot times across all included components.
    pub fn log_comparison_table(&self) {
        klog_info!("================================================================================");
        klog_info!("               INTEGRATE OS MICRO OS BOOT-TIME COMPARISON TABLE                ");
        klog_info!("================================================================================");
        klog_info!("Check  Component Added                             Phase  Boot Time (ms)  Delta ");
        klog_info!("-----  ------------------------------------------  -----  --------------  ------");

        let baseline = self.component_measurements.first().map(|c| c.boot_time_ms).unwrap_or(0);
        for (i, c) in self.component_measurements.iter().enumerate() {
            let delta = if c.boot_time_ms >= baseline {
                alloc::format!("+{} ms", c.boot_time_ms - baseline)
            } else {
                alloc::format!("-{} ms", baseline - c.boot_time_ms)
            };

            klog_info!(
                "#{:<4} {:<42} {:<6} {:<15} {}",
                i + 1,
                c.component_name,
                c.phase_number,
                c.boot_time_ms,
                delta
            );
        }
        klog_info!("================================================================================");
    }
}

pub static BOOT_PROFILER: IrqSpinlock<BootProfiler> = IrqSpinlock::new(BootProfiler::new());

/// Records a boot milestone into the global profiler.
pub fn record_stage(name: &'static str) {
    BOOT_PROFILER.lock().record_stage(name);
}

/// Records a component boot time measurement into the global profiler.
pub fn record_component_boot(component: &'static str, phase: u8, time_ms: u64, notes: &'static str) {
    BOOT_PROFILER.lock().record_component_boot_time(component, phase, time_ms, notes);
}

/// Logs the consolidated boot time comparison table.
pub fn log_boot_comparison_table() {
    BOOT_PROFILER.lock().log_comparison_table();
}
