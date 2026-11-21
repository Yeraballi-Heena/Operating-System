//! Integrate OS — Application Framework Subsystem (Phase 21)
//!
//! Provides the application runtime model, lifecycle management,
//! event bus, clipboard service, and GUI Toolkit window integration:
//! - Polymorphic `Application` trait (`app.rs`)
//! - Execution context & UI root binding (`env.rs`)
//! - Unified event envelope (`event.rs`)
//! - Thread-safe system clipboard (`clipboard.rs`)
//! - Central application manager runtime (`manager.rs`)
//! - Verification test suite (`tests.rs`)

pub mod app;
pub mod clipboard;
pub mod env;
pub mod event;
pub mod manager;
pub mod types;
pub mod tests;

pub use app::Application;
pub use env::AppEnv;
pub use event::AppEvent;
pub use manager::{AppInstance, AppManager};
pub use types::{AppError, AppState, InstanceId};

use crate::klog_ok;
use crate::sync::IrqSpinlock;

pub static APP_MANAGER: IrqSpinlock<Option<AppManager>> = IrqSpinlock::new(None);

/// Initializes the Application Framework subsystem.
pub fn init() {
    let mgr = AppManager::new();
    *APP_MANAGER.lock() = Some(mgr);
    klog_ok!("Application Framework Initialized: App lifecycle, event bus & runtime dispatcher active");
}
