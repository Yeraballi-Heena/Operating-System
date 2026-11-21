//! Integrate OS — Debian Packaging Subsystem (APT / APTITUDE)

pub mod package;
pub mod database;
pub mod manager;
pub mod tests;

use crate::sync::IrqSpinlock;
use crate::pkg::manager::PackageManager;
use crate::klog_ok;

pub static PACKAGE_MANAGER: IrqSpinlock<Option<PackageManager>> = IrqSpinlock::new(None);

/// Initializes the Debian package management layer.
pub fn init() {
    let mut guard = PACKAGE_MANAGER.lock();
    if guard.is_none() {
        *guard = Some(PackageManager::new());
    }
    let mgr = guard.as_ref().unwrap();
    klog_ok!(
        "[pkg] APT/APTITUDE package manager ready ({} packages registered, {} installed)",
        mgr.db.total_count(),
        mgr.db.installed_count()
    );
}
