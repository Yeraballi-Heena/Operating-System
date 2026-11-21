//! Integrate OS — Central Integration Manager & Diagnostic Coordinator
//!
//! Orchestrates the subsystem registry, cross-subsystem pipeline contracts,
//! and global telemetry collection across all 32 subsystems.

use alloc::vec::Vec;

use crate::integration::health::SystemHealthReport;
use crate::integration::matrix::{IntegrationMatrix, PipelineResult};
use crate::integration::subsystems::{SubsystemRegistry, SubsystemState};
use crate::sync::IrqSpinlock;

/// Central coordinator for the entire Integrate OS integration layer.
pub struct IntegrationManager {
    registry: SubsystemRegistry,
    pipeline_results: Vec<PipelineResult>,
    health_report: Option<SystemHealthReport>,
}

impl IntegrationManager {
    /// Creates a new integration manager and discovers all active subsystems.
    pub fn new() -> Self {
        let registry = SubsystemRegistry::new();
        Self {
            registry,
            pipeline_results: Vec::new(),
            health_report: None,
        }
    }

    /// Access the central subsystem registry.
    pub fn registry(&self) -> &SubsystemRegistry {
        &self.registry
    }

    /// Mutably access the subsystem registry.
    pub fn registry_mut(&mut self) -> &mut SubsystemRegistry {
        &mut self.registry
    }

    /// Refreshes all subsystem states and executes all 8 cross-subsystem pipeline verifications.
    pub fn run_diagnostics(&mut self) -> bool {
        self.registry.refresh_states();
        self.pipeline_results = IntegrationMatrix::verify_all();
        let health = SystemHealthReport::collect();
        self.health_report = Some(health);

        let all_healthy = self.registry.verify_all_healthy();
        let all_pipelines_passed = self.pipeline_results.iter().all(|p| p.passed);

        all_healthy && all_pipelines_passed
    }

    /// Returns the cached pipeline results.
    pub fn pipeline_results(&self) -> &[PipelineResult] {
        &self.pipeline_results
    }

    /// Returns the cached system health report.
    pub fn health_report(&self) -> Option<&SystemHealthReport> {
        self.health_report.as_ref()
    }

    /// Emits a structured diagnostic report to the serial log.
    pub fn log_status_report(&self) {
        crate::klog_info!("============================================================");
        crate::klog_info!("          INTEGRATE OS FULL SYSTEM INTEGRATION REPORT        ");
        crate::klog_info!("============================================================");

        crate::klog_info!(
            "Subsystems Tracked: {} / {} Operational ({:.0}%)",
            self.registry.operational_count(),
            self.registry.count(),
            (self.registry.operational_count() as f32 / self.registry.count() as f32) * 100.0
        );

        for sub in self.registry.all() {
            let status_mark = match sub.state {
                SubsystemState::Operational => "[PASS]",
                SubsystemState::Degraded => "[WARN]",
                _ => "[FAIL]",
            };
            crate::klog_info!(
                "  Phase {:02} {:<6} {:<32} : {}",
                sub.phase,
                status_mark,
                sub.name,
                sub.state
            );
        }

        crate::klog_info!("---------------- CROSS-SUBSYSTEM PIPELINES -----------------");
        for p in &self.pipeline_results {
            let mark = if p.passed { "[PASS]" } else { "[FAIL]" };
            crate::klog_info!("  {:<6} {:<40} -> {}", mark, p.name, p.message.as_str());
        }

        if let Some(ref health) = self.health_report {
            health.log_summary();
        }

        crate::klog_info!("============================================================");
    }
}

/// Global system integration manager protected by an interrupt-safe spinlock.
pub static INTEGRATION_MANAGER: IrqSpinlock<Option<IntegrationManager>> = IrqSpinlock::new(None);

/// Executes a closure with an immutable reference to the central integration manager.
pub fn with_integration<R, F: FnOnce(&IntegrationManager) -> R>(f: F) -> Option<R> {
    let guard = INTEGRATION_MANAGER.lock();
    guard.as_ref().map(f)
}

/// Executes a closure with a mutable reference to the central integration manager.
pub fn with_integration_mut<R, F: FnOnce(&mut IntegrationManager) -> R>(f: F) -> Option<R> {
    let mut guard = INTEGRATION_MANAGER.lock();
    guard.as_mut().map(f)
}
