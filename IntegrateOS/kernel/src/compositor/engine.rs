//! Central Compositor Engine & Layer Composition Pipeline
//!
//! Orchestrates the multi-layered rendering pipeline:
//! 1. Background Wallpaper Layer
//! 2. Window Drop Shadows Layer
//! 3. Z-Ordered Window Surfaces Layer
//! 4. Desktop Panel / Taskbar Layer
//! 5. Hardware/Software Cursor Layer
//!
//! Enforces damage-driven incremental VRAM flushes to conserve bus bandwidth.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::input::cursor::VirtualCursor;
use crate::wm::render::{render_desktop, render_window};
use crate::wm::types::*;
use crate::wm::WindowManager;

use crate::compositor::damage::DamageRegion;
use crate::compositor::effects::draw_drop_shadow;
use crate::compositor::taskbar::{Taskbar, TaskbarHitResult};

/// Performance and accounting metrics collected by the compositor.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompositorStats {
    pub total_frames: u64,
    pub dirty_pixels_flushed: u64,
    pub full_redraws: u64,
}

/// The core compositor engine.
pub struct CompositorEngine {
    pub screen_width: u32,
    pub screen_height: u32,
    pub damage: DamageRegion,
    pub taskbar: Taskbar,
    pub shadow_size: u32,
    pub stats: CompositorStats,
}

impl CompositorEngine {
    /// Creates a new compositor engine for display dimensions `(width, height)`.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            screen_width: width,
            screen_height: height,
            damage: DamageRegion::new(width, height),
            taskbar: Taskbar::new(width, height),
            shadow_size: 6,
            stats: CompositorStats {
                total_frames: 0,
                dirty_pixels_flushed: 0,
                full_redraws: 0,
            },
        }
    }

    /// Marks a rectangular screen region as dirty.
    pub fn mark_dirty(&mut self, rect: Rect) {
        self.damage.add_rect(rect);
    }

    /// Marks the full screen area as dirty.
    pub fn mark_fullscreen_dirty(&mut self) {
        self.damage.add_fullscreen();
        self.stats.full_redraws += 1;
    }

    /// Returns `true` if any regions need redrawing.
    pub fn is_dirty(&self) -> bool {
        self.damage.is_dirty()
    }

    /// Handles a mouse click on compositor-managed UI elements (e.g. Taskbar).
    /// Returns `true` if consumed by the compositor.
    pub fn handle_mouse_click(
        &mut self,
        wm: &mut WindowManager,
        px: i32,
        py: i32,
    ) -> bool {
        match self.taskbar.hit_test(px, py, wm) {
            TaskbarHitResult::StartButton => {
                self.taskbar.start_active = !self.taskbar.start_active;
                self.mark_dirty(self.taskbar.bounds());
                true
            }
            TaskbarHitResult::TaskItem(win_id) => {
                if let Some(win) = wm.find_window(win_id) {
                    if win.active && win.state != WindowState::Minimized {
                        let _ = wm.minimize_window(win_id);
                    } else if win.state == WindowState::Minimized {
                        let _ = wm.restore_window(win_id);
                    } else {
                        let _ = wm.focus_window(win_id);
                    }
                }
                self.mark_fullscreen_dirty();
                true
            }
            TaskbarHitResult::Clock => {
                self.mark_dirty(self.taskbar.clock_rect());
                true
            }
            TaskbarHitResult::Nowhere => false,
        }
    }

    /// Executes the full multi-layered compositing pipeline onto `canvas`.
    pub fn compose(
        &mut self,
        canvas: &mut Canvas,
        wm: &WindowManager,
        cursor_pos: (i32, i32),
        uptime_ticks: u64,
    ) {
        // 1. Layer 1: Background Wallpaper
        render_desktop(canvas);

        // 2. Layer 2 & 3: Window Drop Shadows & Window Surfaces (Z-Order: Bottom to Top)
        for win in &wm.windows {
            if win.state == WindowState::Minimized || win.state == WindowState::Hidden {
                continue;
            }
            // Draw window drop shadow if not borderless
            if !win.flags.contains(WindowFlags::BORDERLESS) && self.shadow_size > 0 {
                draw_drop_shadow(canvas, win.bounds, self.shadow_size, win.active);
            }
            // Draw window frame, title bar, controls, and client buffer
            render_window(canvas, win);
        }

        // 3. Layer 4: Desktop Taskbar
        self.taskbar.render(canvas, wm, uptime_ticks);

        // 4. Layer 5: Virtual Mouse Cursor Overlay
        let mut cursor = VirtualCursor::new(canvas.width as i32, canvas.height as i32);
        cursor.set_position(cursor_pos.0, cursor_pos.1);
        cursor.render(|px, py, col| {
            canvas.draw_pixel(px, py, Color32::from_u32(col));
        });

        // 5. Accounting & Dirty VRAM Presentation
        self.stats.total_frames += 1;
        self.stats.dirty_pixels_flushed += self.damage.total_dirty_pixels();

        // Flush dirty regions to physical VRAM
        if let Some(dirty_bbox) = self.damage.bounding_box() {
            canvas.present_rect(dirty_bbox);
        } else {
            // Default full present if damage tracking was unset
            canvas.present();
        }

        self.damage.clear();
    }
}
