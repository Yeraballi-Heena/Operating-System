//! Integrate OS — Compositor & Layer Composition Pipeline
//!
//! Provides:
//! - Incremental damage tracking and dirty rect merging (`damage.rs`)
//! - Software alpha compositing and window drop shadows (`effects.rs`)
//! - Desktop panel / taskbar with active window indicators and system clock (`taskbar.rs`)
//! - Multi-layered compositing pipeline: Background -> Shadows -> Windows -> Taskbar -> Cursor (`engine.rs`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod damage;
pub mod effects;
pub mod engine;
pub mod taskbar;
pub mod tests;

use crate::graphics::canvas::Canvas;
use crate::graphics::rect::Rect;
use crate::klog_ok;
use crate::sync::IrqSpinlock;

pub use crate::compositor::damage::DamageRegion;
pub use crate::compositor::effects::*;
pub use crate::compositor::engine::{CompositorEngine, CompositorStats};
pub use crate::compositor::taskbar::Taskbar;

pub static COMPOSITOR: IrqSpinlock<Option<CompositorEngine>> = IrqSpinlock::new(None);

/// Initializes the Compositor with default display resolution (1024x768).
pub fn init() {
    let comp = CompositorEngine::new(1024, 768);
    *COMPOSITOR.lock() = Some(comp);
    klog_ok!("Compositor Initialized: Multi-layer scene graph & damage tracking active");
}

/// Marks a region of the screen as damaged and requiring recomposition.
pub fn mark_dirty(rect: Rect) {
    if let Some(comp) = COMPOSITOR.lock().as_mut() {
        comp.mark_dirty(rect);
    }
}

/// Marks the entire screen as damaged.
pub fn mark_fullscreen() {
    if let Some(comp) = COMPOSITOR.lock().as_mut() {
        comp.mark_fullscreen_dirty();
    }
}

/// Composes all desktop layers onto `canvas`.
pub fn compose(
    canvas: &mut Canvas,
    wm: &crate::wm::WindowManager,
    cursor_pos: (i32, i32),
    uptime_ticks: u64,
) {
    if let Some(comp) = COMPOSITOR.lock().as_mut() {
        comp.compose(canvas, wm, cursor_pos, uptime_ticks);
    }
}

/// Returns snapshot of compositor performance statistics.
pub fn stats() -> Option<CompositorStats> {
    COMPOSITOR.lock().as_ref().map(|c| c.stats)
}
