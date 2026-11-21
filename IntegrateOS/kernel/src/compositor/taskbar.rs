//! Desktop Panel & Taskbar Compositing
//!
//! Provides the desktop taskbar pinned to the bottom of the screen:
//! - Start menu button
//! - Task list buttons representing open windows with active/minimized visual indicators
//! - System tray clock displaying uptime ticks
//! - Hit-testing for taskbar interaction and window switching

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::FONT_HEIGHT;
use crate::graphics::rect::Rect;
use crate::wm::types::*;
use crate::wm::WindowManager;

pub const TASKBAR_HEIGHT: u32 = 30;
pub const START_BTN_WIDTH: u32 = 90;
pub const TASK_ITEM_WIDTH: u32 = 140;
pub const CLOCK_WIDTH: u32 = 90;

pub const TASKBAR_BG: u32 = 0xFF14171D;
pub const TASKBAR_BORDER: u32 = 0xFF2A2E38;
pub const START_BTN_BG: u32 = 0xFF2B3A4C;
pub const START_BTN_TEXT: u32 = 0xFFFFFFFF;
pub const TASK_ACTIVE_BG: u32 = 0xFF2E3D52;
pub const TASK_INACTIVE_BG: u32 = 0xFF1E222A;
pub const TASK_ACCENT: u32 = 0xFF528BFF;

/// Hit-test result on the taskbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarHitResult {
    Nowhere,
    StartButton,
    TaskItem(WindowId),
    Clock,
}

/// Desktop taskbar component.
#[derive(Debug, Clone)]
pub struct Taskbar {
    pub screen_width: u32,
    pub screen_height: u32,
    pub start_active: bool,
}

impl Taskbar {
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            screen_width: width,
            screen_height: height,
            start_active: false,
        }
    }

    /// Taskbar bounding rectangle.
    pub fn bounds(&self) -> Rect {
        Rect::new(
            0,
            (self.screen_height.saturating_sub(TASKBAR_HEIGHT)) as i32,
            self.screen_width,
            TASKBAR_HEIGHT,
        )
    }

    /// Start button rectangle.
    pub fn start_button_rect(&self) -> Rect {
        let tb = self.bounds();
        Rect::new(tb.x + 4, tb.y + 3, START_BTN_WIDTH, TASKBAR_HEIGHT - 6)
    }

    /// Clock rectangle.
    pub fn clock_rect(&self) -> Rect {
        let tb = self.bounds();
        let cx = tb.right() - (CLOCK_WIDTH + 4) as i32;
        Rect::new(cx, tb.y + 3, CLOCK_WIDTH, TASKBAR_HEIGHT - 6)
    }

    /// Computes the bounding rectangle for window `idx` in the task list.
    pub fn task_item_rect(&self, idx: usize) -> Rect {
        let tb = self.bounds();
        let start_x = tb.x + (START_BTN_WIDTH + 12) as i32;
        let x = start_x + (idx as i32) * ((TASK_ITEM_WIDTH + 4) as i32);
        Rect::new(x, tb.y + 3, TASK_ITEM_WIDTH, TASKBAR_HEIGHT - 6)
    }

    /// Performs hit-testing against taskbar components.
    pub fn hit_test(&self, px: i32, py: i32, wm: &WindowManager) -> TaskbarHitResult {
        if !self.bounds().contains(px, py) {
            return TaskbarHitResult::Nowhere;
        }

        if self.start_button_rect().contains(px, py) {
            return TaskbarHitResult::StartButton;
        }

        if self.clock_rect().contains(px, py) {
            return TaskbarHitResult::Clock;
        }

        for (idx, win) in wm.windows.iter().enumerate() {
            let item_rect = self.task_item_rect(idx);
            if item_rect.contains(px, py) {
                return TaskbarHitResult::TaskItem(win.id);
            }
        }

        TaskbarHitResult::Nowhere
    }

    /// Renders the taskbar, start button, window items, and system clock.
    pub fn render(&self, canvas: &mut Canvas, wm: &WindowManager, uptime_ticks: u64) {
        let tb = self.bounds();

        // 1. Taskbar background & top separator line
        canvas.draw_filled_rect(tb, Color32::from_u32(TASKBAR_BG));
        canvas.draw_hline(tb.x, tb.y, tb.width, Color32::from_u32(TASKBAR_BORDER));

        // 2. Start button
        let sb = self.start_button_rect();
        let sb_color = if self.start_active {
            Color32::from_u32(TASK_ACCENT)
        } else {
            Color32::from_u32(START_BTN_BG)
        };
        canvas.draw_filled_rect(sb, sb_color);
        canvas.draw_rect(sb, Color32::from_u32(TASKBAR_BORDER));
        let sb_ty = sb.y + ((sb.height.saturating_sub(FONT_HEIGHT as u32)) / 2) as i32;
        canvas.draw_text(sb.x + 8, sb_ty, "Integrate", Color32::from_u32(START_BTN_TEXT), None);

        // 3. Task list items
        for (idx, win) in wm.windows.iter().enumerate() {
            let item_rect = self.task_item_rect(idx);
            if item_rect.right() > self.clock_rect().x {
                break; // Do not overlap clock
            }

            let is_active = win.active && win.state != WindowState::Minimized;
            let bg_color = if is_active {
                Color32::from_u32(TASK_ACTIVE_BG)
            } else {
                Color32::from_u32(TASK_INACTIVE_BG)
            };

            canvas.draw_filled_rect(item_rect, bg_color);
            canvas.draw_rect(item_rect, Color32::from_u32(TASKBAR_BORDER));

            if is_active {
                // Active blue accent underline
                canvas.draw_hline(
                    item_rect.x + 2,
                    item_rect.bottom() - 2,
                    item_rect.width - 4,
                    Color32::from_u32(TASK_ACCENT),
                );
            }

            // Window title snippet
            let ty = item_rect.y + ((item_rect.height.saturating_sub(FONT_HEIGHT as u32)) / 2) as i32;
            let text_color = if is_active {
                Color32::WHITE
            } else {
                Color32::from_u32(0xFF8A92A0)
            };

            let title = if win.title.len() > 14 {
                &win.title[..14]
            } else {
                &win.title
            };
            canvas.draw_text(item_rect.x + 6, ty, title, text_color, None);
        }

        // 4. System clock / uptime display
        let cb = self.clock_rect();
        canvas.draw_filled_rect(cb, Color32::from_u32(TASK_INACTIVE_BG));
        canvas.draw_rect(cb, Color32::from_u32(TASKBAR_BORDER));

        // Format uptime into seconds string e.g. "12.3s"
        let seconds = uptime_ticks / 100;
        let tenths = (uptime_ticks % 100) / 10;
        let mut time_str = [b' '; 8];
        let mut pos = 0;
        let s1 = (seconds / 10) % 10;
        let s2 = seconds % 10;
        time_str[pos] = b'0' + s1 as u8; pos += 1;
        time_str[pos] = b'0' + s2 as u8; pos += 1;
        time_str[pos] = b'.'; pos += 1;
        time_str[pos] = b'0' + tenths as u8; pos += 1;
        time_str[pos] = b's'; pos += 1;

        if let Ok(clock_text) = core::str::from_utf8(&time_str[..pos]) {
            let cy = cb.y + ((cb.height.saturating_sub(FONT_HEIGHT as u32)) / 2) as i32;
            canvas.draw_text(cb.x + 12, cy, clock_text, Color32::WHITE, None);
        }
    }
}
