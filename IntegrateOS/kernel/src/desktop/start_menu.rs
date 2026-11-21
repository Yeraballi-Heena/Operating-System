//! Integrate OS — Start Menu Popup Component
//!
//! Provides the application launcher popup anchored above the taskbar Start button,
//! featuring categorized application entries, system power actions, and focus dismissal.

use alloc::vec::Vec;
use crate::compositor::effects::draw_drop_shadow;
use crate::compositor::taskbar::TASKBAR_HEIGHT;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::desktop::types::{AppId, DesktopAction};

pub const MENU_WIDTH: u32 = 224;
pub const MENU_HEIGHT: u32 = 264;
pub const ITEM_HEIGHT: u32 = 30;

/// Result of hit-testing within or outside the Start Menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartMenuHitResult {
    Outside,
    AppItem(AppId),
    Shutdown,
    Reboot,
    Inside,
}

/// An entry in the Start Menu application list.
#[derive(Debug, Clone)]
pub struct StartMenuItem {
    pub app_id: AppId,
    pub label: &'static str,
    pub rect: Rect,
}

/// The desktop Start Menu popup.
#[derive(Debug, Clone)]
pub struct StartMenu {
    pub bounds: Rect,
    pub is_open: bool,
    pub items: Vec<StartMenuItem>,
    pub hover_index: Option<usize>,
    pub shutdown_rect: Rect,
    pub reboot_rect: Rect,
    pub hover_shutdown: bool,
    pub hover_reboot: bool,
}

impl StartMenu {
    /// Creates a new Start Menu anchored to screen dimensions.
    pub fn new(_screen_width: u32, screen_height: u32) -> Self {
        let x = 4;
        let y = (screen_height.saturating_sub(TASKBAR_HEIGHT + MENU_HEIGHT + 4)) as i32;
        let bounds = Rect::new(x, y, MENU_WIDTH, MENU_HEIGHT);

        let mut menu = Self {
            bounds,
            is_open: false,
            items: Vec::new(),
            hover_index: None,
            shutdown_rect: Rect::new(0, 0, 0, 0),
            reboot_rect: Rect::new(0, 0, 0, 0),
            hover_shutdown: false,
            hover_reboot: false,
        };

        menu.init_layout();
        menu
    }

    /// Initializes child element geometry relative to the menu bounds.
    pub fn init_layout(&mut self) {
        self.items.clear();
        let header_h = 44;
        let mut cur_y = self.bounds.y + header_h + 4;

        let app_list = [
            AppId::Terminal,
            AppId::FileManager,
            AppId::SystemMonitor,
            AppId::TextEditor,
            AppId::Settings,
        ];

        for &app in &app_list {
            let rect = Rect::new(self.bounds.x + 6, cur_y, self.bounds.width.saturating_sub(12), ITEM_HEIGHT);
            self.items.push(StartMenuItem {
                app_id: app,
                label: app.title(),
                rect,
            });
            cur_y += ITEM_HEIGHT as i32 + 2;
        }

        // Bottom system actions
        let bottom_y = self.bounds.bottom() - 36;
        let btn_w = (self.bounds.width.saturating_sub(18)) / 2;
        self.shutdown_rect = Rect::new(self.bounds.x + 6, bottom_y, btn_w, 28);
        self.reboot_rect = Rect::new(self.bounds.x + 6 + btn_w as i32 + 6, bottom_y, btn_w, 28);
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.hover_index = None;
        self.hover_shutdown = false;
        self.hover_reboot = false;
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Updates hover states based on mouse position.
    pub fn update_hover(&mut self, x: i32, y: i32) {
        if !self.is_open {
            return;
        }

        self.hover_index = None;
        for (i, item) in self.items.iter().enumerate() {
            if item.rect.contains(x, y) {
                self.hover_index = Some(i);
                break;
            }
        }

        self.hover_shutdown = self.shutdown_rect.contains(x, y);
        self.hover_reboot = self.reboot_rect.contains(x, y);
    }

    /// Hit tests a screen point against the Start Menu.
    pub fn hit_test(&self, x: i32, y: i32) -> StartMenuHitResult {
        if !self.is_open {
            return StartMenuHitResult::Outside;
        }

        if !self.bounds.contains(x, y) {
            return StartMenuHitResult::Outside;
        }

        for item in &self.items {
            if item.rect.contains(x, y) {
                return StartMenuHitResult::AppItem(item.app_id);
            }
        }

        if self.shutdown_rect.contains(x, y) {
            return StartMenuHitResult::Shutdown;
        }

        if self.reboot_rect.contains(x, y) {
            return StartMenuHitResult::Reboot;
        }

        StartMenuHitResult::Inside
    }

    /// Handles a click on the Start Menu.
    pub fn handle_click(&mut self, x: i32, y: i32) -> DesktopAction {
        match self.hit_test(x, y) {
            StartMenuHitResult::Outside => {
                self.close();
                DesktopAction::None
            }
            StartMenuHitResult::AppItem(app) => {
                self.close();
                DesktopAction::LaunchApp(app)
            }
            StartMenuHitResult::Shutdown => {
                self.close();
                DesktopAction::Shutdown
            }
            StartMenuHitResult::Reboot => {
                self.close();
                DesktopAction::Reboot
            }
            StartMenuHitResult::Inside => {
                DesktopAction::None
            }
        }
    }

    /// Renders the Start Menu popup with drop shadow onto `canvas`.
    pub fn render(&self, canvas: &mut Canvas) {
        if !self.is_open {
            return;
        }

        // 1. Soft drop shadow around the menu card
        draw_drop_shadow(canvas, self.bounds, 6, true);

        // 2. Card background & border
        canvas.draw_filled_rect(self.bounds, Color32::from_u32(BG_PANEL));
        canvas.draw_rect(self.bounds, Color32::from_u32(BORDER_HOVER));

        // 3. Header Banner
        let header_rect = Rect::new(self.bounds.x + 1, self.bounds.y + 1, self.bounds.width - 2, 40);
        canvas.draw_filled_rect(header_rect, Color32::from_u32(0xFF1E2633));
        canvas.draw_text(self.bounds.x + 12, self.bounds.y + 8, "Integrate OS", Color32::from_u32(TEXT_PRIMARY), None);
        canvas.draw_text(self.bounds.x + 12, self.bounds.y + 24, "v0.1.0 (x86_64)", Color32::from_u32(TEXT_SECONDARY), None);

        // Separator below header
        canvas.draw_hline(self.bounds.x + 1, self.bounds.y + 42, self.bounds.width - 2, Color32::from_u32(BORDER_DEFAULT));

        // 4. Application items list
        for (i, item) in self.items.iter().enumerate() {
            let is_hover = self.hover_index == Some(i);
            if is_hover {
                canvas.draw_filled_rect(item.rect, Color32::from_u32(0xFF283648));
                canvas.draw_rect(item.rect, Color32::from_u32(ACCENT_PRIMARY));
            }

            // Bullet / Mini icon indicator
            let bullet_x = item.rect.x + 8;
            let bullet_y = item.rect.y + 11;
            canvas.draw_filled_rect(Rect::new(bullet_x, bullet_y, 8, 8), Color32::from_u32(ACCENT_PRIMARY));

            // App Label text
            let text_x = item.rect.x + 24;
            let text_y = item.rect.y + 7;
            let text_col = if is_hover { TEXT_PRIMARY } else { 0xFFD1D5DB };
            canvas.draw_text(text_x, text_y, item.label, Color32::from_u32(text_col), None);
        }

        // 5. Separator above bottom buttons
        canvas.draw_hline(self.bounds.x + 1, self.shutdown_rect.y - 6, self.bounds.width - 2, Color32::from_u32(BORDER_DEFAULT));

        // 6. Shutdown button
        let sd_bg = if self.hover_shutdown { BTN_DANGER_HOVER } else { BTN_DANGER_BG };
        canvas.draw_filled_rect(self.shutdown_rect, Color32::from_u32(sd_bg));
        canvas.draw_rect(self.shutdown_rect, Color32::from_u32(BORDER_DEFAULT));
        let sd_text_x = self.shutdown_rect.x + (self.shutdown_rect.width.saturating_sub(8 * (FONT_WIDTH as u32)) / 2) as i32;
        let sd_text_y = self.shutdown_rect.y + (self.shutdown_rect.height.saturating_sub(FONT_HEIGHT as u32) / 2) as i32;
        canvas.draw_text(sd_text_x, sd_text_y, "Shutdown", Color32::from_u32(TEXT_PRIMARY), None);

        // 7. Reboot button
        let rb_bg = if self.hover_reboot { BTN_SECONDARY_HOVER } else { BTN_SECONDARY_BG };
        canvas.draw_filled_rect(self.reboot_rect, Color32::from_u32(rb_bg));
        canvas.draw_rect(self.reboot_rect, Color32::from_u32(BORDER_DEFAULT));
        let rb_text_x = self.reboot_rect.x + (self.reboot_rect.width.saturating_sub(6 * (FONT_WIDTH as u32)) / 2) as i32;
        let rb_text_y = self.reboot_rect.y + (self.reboot_rect.height.saturating_sub(FONT_HEIGHT as u32) / 2) as i32;
        canvas.draw_text(rb_text_x, rb_text_y, "Reboot", Color32::from_u32(TEXT_PRIMARY), None);
    }
}
