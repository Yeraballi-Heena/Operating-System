//! Integrate OS — Desktop Icons & Grid Management
//!
//! Provides desktop icons with vector graphics, text labels, selection/hover states,
//! and single-click selection / double-click application launching.

use alloc::vec::Vec;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;
use crate::desktop::types::{AppId, DesktopAction, IconState};

pub const ICON_W: u32 = 76;
pub const ICON_H: u32 = 82;
pub const ICON_SPACING: u32 = 16;
pub const DOUBLE_CLICK_TICKS: u64 = 50; // 500 ms at 100 Hz timer

/// An individual desktop icon.
#[derive(Debug, Clone)]
pub struct DesktopIcon {
    pub app_id: AppId,
    pub title: &'static str,
    pub bounds: Rect,
    pub state: IconState,
    pub last_click_tick: u64,
}

impl DesktopIcon {
    pub fn new(app_id: AppId, x: i32, y: i32) -> Self {
        Self {
            app_id,
            title: app_id.title(),
            bounds: Rect::new(x, y, ICON_W, ICON_H),
            state: IconState::Normal,
            last_click_tick: 0,
        }
    }
}

/// Collection of desktop icons arranged in a grid.
#[derive(Debug, Clone)]
pub struct DesktopIconGrid {
    pub icons: Vec<DesktopIcon>,
}

impl DesktopIconGrid {
    /// Creates the standard default desktop icon grid.
    pub fn new() -> Self {
        let mut grid = Self { icons: Vec::new() };
        grid.init_defaults();
        grid
    }

    /// Initializes standard desktop shortcut icons in the left column (4 icons for Phase 20 tests).
    pub fn init_defaults(&mut self) {
        self.icons.clear();
        let start_x = 24;
        let mut cur_y = 24;

        let apps = [
            AppId::Terminal,
            AppId::FileManager,
            AppId::SystemMonitor,
            AppId::Settings,
        ];

        for &app in &apps {
            self.icons.push(DesktopIcon::new(app, start_x, cur_y));
            cur_y += (ICON_H + ICON_SPACING) as i32;
        }
    }

    /// Initializes the full 8-icon live desktop grid in two balanced columns.
    pub fn init_live_desktop(&mut self) {
        self.icons.clear();

        let col1_apps = [
            AppId::Pc,
            AppId::Terminal,
            AppId::FileManager,
            AppId::SystemMonitor,
            AppId::PackageManager,
        ];

        let col2_apps = [
            AppId::Settings,
            AppId::Games,
            AppId::MediaPlayer,
        ];

        let mut cur_y = 18;
        for &app in &col1_apps {
            self.icons.push(DesktopIcon::new(app, 16, cur_y));
            cur_y += (ICON_H + 10) as i32;
        }

        let mut cur_y2 = 18;
        for &app in &col2_apps {
            self.icons.push(DesktopIcon::new(app, 116, cur_y2));
            cur_y2 += (ICON_H + 10) as i32;
        }
    }

    /// Hit tests a screen point against all icons.
    pub fn hit_test(&self, x: i32, y: i32) -> Option<usize> {
        for (i, icon) in self.icons.iter().enumerate() {
            if icon.bounds.contains(x, y) {
                return Some(i);
            }
        }
        None
    }

    /// Updates hover states based on mouse position.
    pub fn update_hover(&mut self, x: i32, y: i32) {
        for icon in &mut self.icons {
            if icon.bounds.contains(x, y) {
                if icon.state == IconState::Normal {
                    icon.state = IconState::Hover;
                }
            } else if icon.state == IconState::Hover {
                icon.state = IconState::Normal;
            }
        }
    }

    /// Handles a mouse click on the desktop (double-click launch semantics for Phase 20 test suite).
    pub fn handle_click(&mut self, x: i32, y: i32, tick: u64) -> DesktopAction {
        if let Some(hit_idx) = self.hit_test(x, y) {
            let icon = &mut self.icons[hit_idx];
            let is_double_click = icon.state == IconState::Selected
                && (tick.saturating_sub(icon.last_click_tick) <= DOUBLE_CLICK_TICKS);

            if is_double_click {
                icon.last_click_tick = 0;
                return DesktopAction::LaunchApp(icon.app_id);
            } else {
                icon.state = IconState::Selected;
                icon.last_click_tick = tick;

                // Deselect other icons
                for (i, other) in self.icons.iter_mut().enumerate() {
                    if i != hit_idx {
                        other.state = IconState::Normal;
                    }
                }
                return DesktopAction::None;
            }
        } else {
            // Clicked empty desktop background: deselect all
            for icon in &mut self.icons {
                icon.state = IconState::Normal;
            }
            DesktopAction::None
        }
    }

    /// Handles a mouse click in the live interactive desktop: single click selects AND launches/focuses the app.
    pub fn handle_live_click(&mut self, x: i32, y: i32, tick: u64) -> DesktopAction {
        if let Some(hit_idx) = self.hit_test(x, y) {
            let app_id = self.icons[hit_idx].app_id;
            for (i, icon) in self.icons.iter_mut().enumerate() {
                if i == hit_idx {
                    icon.state = IconState::Selected;
                    icon.last_click_tick = tick;
                } else {
                    icon.state = IconState::Normal;
                }
            }
            DesktopAction::LaunchApp(app_id)
        } else {
            for icon in &mut self.icons {
                icon.state = IconState::Normal;
            }
            DesktopAction::None
        }
    }

    /// Renders all desktop icons onto `canvas`.
    pub fn render(&self, canvas: &mut Canvas) {
        for icon in &self.icons {
            // 1. Selection / Hover background highlight
            if icon.state == IconState::Selected {
                canvas.draw_filled_rect(icon.bounds, Color32::rgba(59, 130, 246, 75));
                canvas.draw_rect(icon.bounds, Color32::from_u32(0xFF528BFF));
            } else if icon.state == IconState::Hover {
                canvas.draw_filled_rect(icon.bounds, Color32::rgba(255, 255, 255, 25));
                canvas.draw_rect(icon.bounds, Color32::rgba(255, 255, 255, 60));
            }

            // 2. Icon Graphic (36x36 at top center of icon bounds)
            let icon_cx = icon.bounds.x + (icon.bounds.width as i32) / 2;
            let icon_top_y = icon.bounds.y + 6;
            let icon_rect = Rect::new(icon_cx - 18, icon_top_y, 36, 36);

            match icon.app_id {
                AppId::Pc => {
                    // Computer Monitor + Mounted SSD Drive Emblem
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 2, icon_rect.y + 2, 32, 22), Color32::from_u32(0xFF1E3A8A));
                    canvas.draw_rect(Rect::new(icon_rect.x + 2, icon_rect.y + 2, 32, 22), Color32::from_u32(0xFF60A5FA));
                    // Screen inner glow + SSD activity bar
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 6, icon_rect.y + 6, 24, 14), Color32::from_u32(0xFF0F172A));
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 8, icon_rect.y + 10, 14, 4), Color32::from_u32(0xFF10B981));
                    // Monitor stand & SSD base
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 14, icon_rect.y + 24, 8, 4), Color32::from_u32(0xFF94A3B8));
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 6, icon_rect.y + 28, 24, 6), Color32::from_u32(0xFF334155));
                    canvas.draw_filled_circle(icon_rect.x + 25, icon_rect.y + 31, 2, Color32::from_u32(0xFF38BDF8));
                }
                AppId::Terminal => {
                    // Dark terminal box
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF1E222A));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFF425670));
                    // Terminal prompt: `>_`
                    let prompt_col = Color32::from_u32(0xFF4CAF50);
                    canvas.draw_line(icon_rect.x + 8, icon_rect.y + 12, icon_rect.x + 14, icon_rect.y + 18, prompt_col);
                    canvas.draw_line(icon_rect.x + 14, icon_rect.y + 18, icon_rect.x + 8, icon_rect.y + 24, prompt_col);
                    canvas.draw_hline(icon_rect.x + 17, icon_rect.y + 24, 10, prompt_col);
                }
                AppId::FileManager => {
                    // Blue folder shape
                    let folder_bg = Color32::from_u32(0xFF3B82F6);
                    let folder_tab = Color32::from_u32(0xFF60A5FA);
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 2, icon_rect.y + 4, 14, 8), folder_tab);
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 2, icon_rect.y + 10, 32, 22), folder_bg);
                    canvas.draw_rect(Rect::new(icon_rect.x + 2, icon_rect.y + 10, 32, 22), Color32::from_u32(0xFF1D4ED8));
                }
                AppId::SystemMonitor => {
                    // Dark panel with ECG / pulse waveform
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF14171D));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFF2A2E38));
                    let wave_col = Color32::from_u32(0xFF10B981);
                    let my = icon_rect.y + 18;
                    canvas.draw_hline(icon_rect.x + 4, my, 6, wave_col);
                    canvas.draw_line(icon_rect.x + 10, my, icon_rect.x + 14, my - 10, wave_col);
                    canvas.draw_line(icon_rect.x + 14, my - 10, icon_rect.x + 18, my + 10, wave_col);
                    canvas.draw_line(icon_rect.x + 18, my + 10, icon_rect.x + 22, my, wave_col);
                    canvas.draw_hline(icon_rect.x + 22, my, 10, wave_col);
                }
                AppId::PackageManager => {
                    // Orange APT/Snap Package Box
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFFD97706));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFFFBBF24));
                    canvas.draw_vline(icon_rect.x + 18, icon_rect.y + 2, 32, Color32::from_u32(0xFFFEF3C7));
                    canvas.draw_hline(icon_rect.x + 2, icon_rect.y + 14, 32, Color32::from_u32(0xFFFEF3C7));
                }
                AppId::Settings => {
                    // Settings gear icon
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF222630));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFF4B5563));
                    let gear_col = Color32::from_u32(0xFF9CA3AF);
                    let gcx = icon_rect.x + 18;
                    let gcy = icon_rect.y + 18;
                    canvas.draw_circle(gcx, gcy, 8, gear_col);
                    canvas.draw_circle(gcx, gcy, 3, gear_col);
                    canvas.draw_vline(gcx, gcy - 12, 5, gear_col);
                    canvas.draw_vline(gcx, gcy + 8, 5, gear_col);
                    canvas.draw_hline(gcx - 12, gcy, 5, gear_col);
                    canvas.draw_hline(gcx + 8, gcy, 5, gear_col);
                }
                AppId::TextEditor => {
                    // Notepad / pencil
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF1F2937));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFF4B5563));
                    let line_col = Color32::from_u32(0xFFE5E7EB);
                    canvas.draw_hline(icon_rect.x + 6, icon_rect.y + 10, 24, line_col);
                    canvas.draw_hline(icon_rect.x + 6, icon_rect.y + 16, 20, line_col);
                    canvas.draw_hline(icon_rect.x + 6, icon_rect.y + 22, 16, line_col);
                }
                AppId::Games => {
                    // Arcade Game Controller Icon (Purple/Magenta badge with D-pad & action buttons)
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF6B21A8));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFFA855F7));
                    // D-pad cross on left
                    let dpad_col = Color32::from_u32(0xFFE9D5FF);
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 6, icon_rect.y + 14, 12, 4), dpad_col);
                    canvas.draw_filled_rect(Rect::new(icon_rect.x + 10, icon_rect.y + 10, 4, 12), dpad_col);
                    // Action buttons (red and yellow dots on right)
                    canvas.draw_filled_circle(icon_rect.x + 24, icon_rect.y + 13, 2, Color32::from_u32(0xFFEF4444));
                    canvas.draw_filled_circle(icon_rect.x + 28, icon_rect.y + 19, 2, Color32::from_u32(0xFFFBBF24));
                }
                AppId::MediaPlayer => {
                    // Media Player Icon (Deep Cyan box with Play triangle and sound waves)
                    canvas.draw_filled_rect(icon_rect, Color32::from_u32(0xFF0E7490));
                    canvas.draw_rect(icon_rect, Color32::from_u32(0xFF06B6D4));
                    // Play Triangle
                    let play_col = Color32::from_u32(0xFFFFFFFF);
                    for i in 0..10 {
                        canvas.draw_vline(icon_rect.x + 12 + i, icon_rect.y + 11 + (i / 2), 14 - i as u32, play_col);
                    }
                    // Equalizer bars on right
                    canvas.draw_vline(icon_rect.x + 25, icon_rect.y + 14, 8, Color32::from_u32(0xFF22D3EE));
                    canvas.draw_vline(icon_rect.x + 28, icon_rect.y + 10, 14, Color32::from_u32(0xFF38BDF8));
                }
            }

            // 3. Label text rendered underneath
            let label_len = icon.title.len() as u32;
            let text_w = label_len * (FONT_WIDTH as u32);
            let text_x = icon.bounds.x + (icon.bounds.width.saturating_sub(text_w) / 2) as i32;
            let text_y = icon.bounds.y + (icon.bounds.height.saturating_sub(FONT_HEIGHT as u32)) as i32 - 4;

            canvas.draw_text(text_x + 1, text_y + 1, icon.title, Color32::from_u32(0xFF000000), None);
            canvas.draw_text(text_x, text_y, icon.title, Color32::from_u32(0xFFFFFFFF), None);
        }
    }
}
