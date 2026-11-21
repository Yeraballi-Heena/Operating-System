//! Integrate OS — Window Manager & Desktop Compositor Subsystem
//!
//! Provides:
//! - Window definitions, bounds, states, and client surface buffers (`window.rs`)
//! - Hit-testing, control buttons, and resizing handles (`types.rs`)
//! - Z-ordered stacking, focus management, drag-move, and resize (`manager.rs`)
//! - Visual styling, colors, and title bar gradients (`theme.rs`)
//! - High-performance back-to-front scene rendering (`render.rs`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod manager;
pub mod render;
pub mod tests;
pub mod theme;
pub mod types;
pub mod window;

use crate::graphics::canvas::Canvas;
use crate::klog_ok;
use crate::sync::IrqSpinlock;
pub use crate::wm::manager::WindowManager;
pub use crate::wm::types::*;
pub use crate::wm::window::Window;

pub static WINDOW_MANAGER: IrqSpinlock<Option<WindowManager>> = IrqSpinlock::new(None);

/// Initializes the Window Manager with screen bounds (1024x768) and default system windows.
pub fn init() {
    let mut wm = WindowManager::new(1024, 768);

    // Create default system desktop windows
    let sys_info_id = wm.create_window(
        "System Information",
        60,
        60,
        420,
        280,
        WindowFlags::DEFAULT,
    );
    if let Some(win) = wm.find_window_mut(sys_info_id) {
        win.fill_client(0xFF1E2228);
        win.draw_client_string(16, 16, "Integrate OS Desktop v0.1.0", 0xFF61AFEF, 0xFF1E2228);
        win.draw_client_string(16, 40, "Kernel Architecture: x86_64 Long Mode", 0xFFABB2BF, 0xFF1E2228);
        win.draw_client_string(16, 60, "Language           : Pure Rust (#![no_std])", 0xFF98C379, 0xFF1E2228);
        win.draw_client_string(16, 80, "Resolution         : 1024x768 @ 32bpp RGBA", 0xFFE5C07B, 0xFF1E2228);
        win.draw_client_string(16, 100, "Window Manager     : Z-Ordered Stacking", 0xFFD19A66, 0xFF1E2228);
        win.draw_client_string(16, 120, "Compositor Mode    : Double-Buffered", 0xFFC678DD, 0xFF1E2228);
    }

    *WINDOW_MANAGER.lock() = Some(wm);
    klog_ok!("Window Manager Initialized: Z-order compositor active, desktop ready");
}

/// Creates a new window in the global window manager.
pub fn create_window(
    title: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    flags: WindowFlags,
) -> Result<WindowId, &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    Ok(wm.create_window(title, x, y, width, height, flags))
}

/// Destroys a window in the global window manager.
pub fn destroy_window(id: WindowId) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.destroy_window(id)
}

/// Focuses a window in the global window manager.
pub fn focus_window(id: WindowId) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.focus_window(id)
}

/// Minimizes a window in the global window manager.
pub fn minimize_window(id: WindowId) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.minimize_window(id)
}

/// Maximizes a window in the global window manager.
pub fn maximize_window(id: WindowId) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.maximize_window(id)
}

/// Restores a window in the global window manager.
pub fn restore_window(id: WindowId) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.restore_window(id)
}

/// Moves a window to `(x, y)` in the global window manager.
pub fn move_window(id: WindowId, x: i32, y: i32) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.move_window(id, x, y)
}

/// Resizes a window in the global window manager.
pub fn resize_window(id: WindowId, width: u32, height: u32) -> Result<(), &'static str> {
    let mut lock = WINDOW_MANAGER.lock();
    let wm = lock.as_mut().ok_or("Window manager not initialized")?;
    wm.resize_window(id, width, height)
}

/// Renders the complete desktop and all managed windows onto `canvas`.
pub fn render(canvas: &mut Canvas) -> Result<(), &'static str> {
    let lock = WINDOW_MANAGER.lock();
    let wm = lock.as_ref().ok_or("Window manager not initialized")?;
    wm.render(canvas);
    Ok(())
}
