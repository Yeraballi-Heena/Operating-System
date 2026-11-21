//! Integrate OS — Full System Integration Layer (Phase 33)
//!
//! Provides:
//! - Central catalog and health tracker for all 32 subsystems (`subsystems.rs`)
//! - Cross-subsystem integration matrix and pipeline contracts (`matrix.rs`)
//! - Global real-time kernel telemetry collector (`health.rs`)
//! - Central integration manager and diagnostics coordinator (`manager.rs`)
//! - End-to-end full system integration test suite (`suite.rs`)

pub mod health;
pub mod manager;
pub mod matrix;
pub mod subsystems;
pub mod suite;

use core::sync::atomic::{AtomicBool, Ordering};

pub use health::SystemHealthReport;
pub use manager::{with_integration, with_integration_mut, IntegrationManager, INTEGRATION_MANAGER};
pub use matrix::{IntegrationMatrix, PipelineId, PipelineResult};
pub use subsystems::{SubsystemCategory, SubsystemId, SubsystemInfo, SubsystemRegistry, SubsystemState};
pub use suite::run_integration_tests;

use crate::klog_ok;

static INTEGRATION_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns true if the full system integration subsystem has been initialized.
#[inline(always)]
pub fn is_integration_initialized() -> bool {
    INTEGRATION_INITIALIZED.load(Ordering::Acquire)
}

/// Initializes the central integration subsystem, runs full diagnostics, and logs the report.
pub fn init() {
    let mut mgr = IntegrationManager::new();
    let diagnostics_ok = mgr.run_diagnostics();

    mgr.log_status_report();

    *INTEGRATION_MANAGER.lock() = Some(mgr);
    INTEGRATION_INITIALIZED.store(true, Ordering::Release);

    if diagnostics_ok {
        klog_ok!("Full System Integration Layer Initialized: 32/32 subsystems operational & all pipeline contracts verified");
    } else {
        crate::klog_info!("Full System Integration Layer Initialized with warnings: Some subsystems degraded or non-critical pipeline notices");
    }
}
