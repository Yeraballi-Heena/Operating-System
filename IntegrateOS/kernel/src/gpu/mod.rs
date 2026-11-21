//! Integrate OS — GPU Foundation Architecture
//!
//! Provides PCI display adapter identification, vendor detection, 2D graphics hardware
//! abstraction, display mode switching, offscreen surfaces, and command queues.

pub mod command;
pub mod device;
pub mod drivers;
pub mod engine;
pub mod manager;
pub mod surface;
pub mod tests;
pub mod traits;
pub mod types;

pub use command::GpuCommand;
pub use device::GpuDevice;
pub use engine::GpuEngine;
pub use manager::{with_gpu_mut, GpuManager, GPU_MANAGER};
pub use surface::{GpuSurface, SurfaceId, SurfaceManager};
pub use traits::GpuDriver;
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static GPU_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the GPU foundation subsystem has initialized.
pub fn is_gpu_initialized() -> bool {
    GPU_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the GPU subsystem, discovers PCI display adapters, and activates the GPU driver.
pub fn init() {
    let mut mgr = GpuManager::new();
    let result = mgr.discover_and_init();

    if let Err(e) = result {
        crate::klog_info!("[gpu] Hardware GPU init notice: {}", e.as_str());
    }

    *GPU_MANAGER.lock() = Some(mgr);
    GPU_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[gpu] GPU Foundation Subsystem initialized");
}
