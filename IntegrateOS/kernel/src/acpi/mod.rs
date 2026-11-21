//! Integrate OS — ACPI Subsystem Architecture
//!
//! Provides discovery and parsing of the Root System Description Pointer (RSDP),
//! standard System Description Tables (RSDT, XSDT, FADT, MADT), power management,
//! system reboot, and ACPI shutdown capabilities.

pub mod manager;
pub mod power;
pub mod rsdp;
pub mod sdt;
pub mod tables;
pub mod tests;
pub mod types;

pub use manager::{power_off, reboot, with_acpi, AcpiManager, ACPI_MANAGER};
pub use power::AcpiPowerManager;
pub use rsdp::find_rsdp;
pub use sdt::{Rsdp, SdtHeader};
pub use tables::{AcpiTables, Fadt, Madt, TableEntry};
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static ACPI_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the ACPI subsystem has initialized.
pub fn is_acpi_initialized() -> bool {
    ACPI_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the ACPI subsystem, discovers the RSDP, and activates power management.
pub fn init() {
    let mut mgr = AcpiManager::new();
    let result = mgr.init();

    if let Err(e) = result {
        crate::klog_info!("[acpi] Hardware ACPI init notice: {}", e.as_str());
    }

    *ACPI_MANAGER.lock() = Some(mgr);
    ACPI_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[acpi] ACPI Subsystem initialized");
}
