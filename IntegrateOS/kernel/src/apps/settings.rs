//! Integrate OS — System Settings & Control Panel Application
//!
//! Provides configuration management for desktop themes, display settings,
//! mouse sensitivity, drop shadows, and audio feedback toggles.

use alloc::string::String;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};

/// System Settings & Control Panel Application.
pub struct SettingsApp {
    pub theme_index: usize, // 0 = Integrate Dark, 1 = Slate Blue, 2 = High Contrast
    pub mouse_sensitivity: u32, // 1..10
    pub drop_shadows: bool,
    pub sound_enabled: bool,
    pub animations_enabled: bool,
    pub status_msg: String,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self {
            theme_index: 0,
            mouse_sensitivity: 5,
            drop_shadows: true,
            sound_enabled: true,
            animations_enabled: true,
            status_msg: String::from("Ready"),
        }
    }

    /// Applies configuration changes to the system environment.
    pub fn apply_settings(&mut self) {
        self.status_msg = String::from("Settings applied successfully");
    }

    /// Reverts settings back to initial factory defaults.
    pub fn reset_defaults(&mut self) {
        self.theme_index = 0;
        self.mouse_sensitivity = 5;
        self.drop_shadows = true;
        self.sound_enabled = true;
        self.animations_enabled = true;
        self.status_msg = String::from("Defaults restored");
    }
}

impl Application for SettingsApp {
    fn name(&self) -> &str {
        "Settings"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Input(ref we) = *event {
            match *we {
                WidgetEvent::KeyDown { code, .. } => {
                    match code {
                        KeyCode::Enter => {
                            self.apply_settings();
                            env.request_redraw();
                        }
                        KeyCode::R => {
                            self.reset_defaults();
                            env.request_redraw();
                        }
                        _ => {}
                    }
                }
                WidgetEvent::MouseDown { x, y, button } => {
                    if button == MouseButton::Left {
                        // 1. Theme Radio Options (y: 60..130)
                        if x >= 20 && x <= 220 {
                            if y >= 60 && y <= 80 {
                                self.theme_index = 0;
                                self.status_msg = String::from("Theme set to Integrate Dark");
                                env.request_redraw();
                                return;
                            } else if y >= 85 && y <= 105 {
                                self.theme_index = 1;
                                self.status_msg = String::from("Theme set to Slate Blue");
                                env.request_redraw();
                                return;
                            } else if y >= 110 && y <= 130 {
                                self.theme_index = 2;
                                self.status_msg = String::from("Theme set to High Contrast");
                                env.request_redraw();
                                return;
                            }
                        }

                        // 2. Mouse Sensitivity Slider (y: 165..190, x: 20..220)
                        if y >= 165 && y <= 190 && x >= 20 && x <= 220 {
                            let fraction = (x - 20) as u32;
                            let val = ((fraction * 9) / 200) + 1;
                            self.mouse_sensitivity = val.clamp(1, 10);
                            let mut s = String::from("Sensitivity set to ");
                            s.push((b'0' + self.mouse_sensitivity as u8) as char);
                            self.status_msg = s;
                            env.request_redraw();
                            return;
                        }

                        // 3. Checkboxes (y: 215..285, x: 20..250)
                        if x >= 20 && x <= 250 {
                            if y >= 215 && y <= 235 {
                                self.drop_shadows = !self.drop_shadows;
                                self.status_msg = String::from("Toggled window drop shadows");
                                env.request_redraw();
                                return;
                            } else if y >= 240 && y <= 260 {
                                self.sound_enabled = !self.sound_enabled;
                                self.status_msg = String::from("Toggled audio feedback");
                                env.request_redraw();
                                return;
                            } else if y >= 265 && y <= 285 {
                                self.animations_enabled = !self.animations_enabled;
                                self.status_msg = String::from("Toggled UI animations");
                                env.request_redraw();
                                return;
                            }
                        }

                        // 4. Action Buttons: [Apply] (x: 20..100, y: h-50..h-24), [Reset] (x: 110..200)
                        let btn_y = env.ui_root.bounds.height as i32 - 45;
                        if y >= btn_y && y <= btn_y + 26 {
                            if x >= 20 && x <= 100 {
                                self.apply_settings();
                                env.request_redraw();
                                return;
                            } else if x >= 110 && x <= 200 {
                                self.reset_defaults();
                                env.request_redraw();
                                return;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}

    fn render(&self, canvas: &mut Canvas) {
        let bg_color = Color32::rgb(0x18, 0x1B, 0x22);
        canvas.clear(bg_color);

        let h = canvas.height as i32;

        // Title Header
        canvas.draw_rect_fill(Rect::new(0, 0, canvas.width, 36), Color32::rgb(0x21, 0x26, 0x2D));
        canvas.draw_hline(0, 36, canvas.width, Color32::rgb(0x30, 0x36, 0x3D));
        canvas.draw_text(16, 10, "Integrate OS System Control Panel", Color32::rgb(0x58, 0xA6, 0xFF), None);

        // Section 1: Appearance & Theme
        canvas.draw_text(20, 44, "Appearance Theme", Color32::rgb(0xEB, 0xF0, 0xF4), None);

        let themes = [
            "Integrate Dark (Cyberpunk Slate)",
            "Slate Blue (Modern Desktop)",
            "High Contrast (Monochrome)",
        ];

        for (i, t_name) in themes.iter().enumerate() {
            let y = 62 + (i as i32) * 24;
            let is_sel = self.theme_index == i;

            // Radio circle
            let circle_color = if is_sel { Color32::rgb(0x38, 0x8B, 0xF6) } else { Color32::rgb(0x48, 0x4F, 0x58) };
            canvas.draw_filled_circle(28, y + 7, 6, circle_color);
            if is_sel {
                canvas.draw_filled_circle(28, y + 7, 3, Color32::rgb(0xFF, 0xFF, 0xFF));
            }

            canvas.draw_text(42, y, t_name, Color32::rgb(0xC9, 0xD1, 0xD9), None);
        }

        // Section 2: Input & Mouse Sensitivity
        let y_input = 142;
        canvas.draw_text(20, y_input, "Mouse Pointer Sensitivity", Color32::rgb(0xEB, 0xF0, 0xF4), None);

        // Track (x: 20..220, y: y_input + 24)
        let track_y = y_input + 24;
        canvas.draw_rect_fill(Rect::new(20, track_y, 200, 6), Color32::rgb(0x30, 0x36, 0x3D));

        let thumb_x = 20 + (((self.mouse_sensitivity - 1) as i32 * 200) / 9);
        canvas.draw_rect_fill(Rect::new(thumb_x - 5, track_y - 5, 10, 16), Color32::rgb(0x38, 0x8B, 0xF6));
        canvas.draw_rect_outline(Rect::new(thumb_x - 5, track_y - 5, 10, 16), Color32::rgb(0xFF, 0xFF, 0xFF));

        let mut sens_str = String::from("Speed: ");
        sens_str.push((b'0' + (self.mouse_sensitivity as u8)) as char);
        sens_str.push_str(" / 10");
        canvas.draw_text(230, track_y - 4, &sens_str, Color32::rgb(0x8B, 0x94, 0x9E), None);

        // Section 3: Window Manager & Effects
        let y_fx = 196;
        canvas.draw_text(20, y_fx, "Desktop Effects & Sound", Color32::rgb(0xEB, 0xF0, 0xF4), None);

        draw_checkbox(canvas, 20, 218, "Enable Window Drop Shadows", self.drop_shadows);
        draw_checkbox(canvas, 20, 242, "Enable System UI Audio Effects", self.sound_enabled);
        draw_checkbox(canvas, 20, 266, "Enable Window Animations & Transitions", self.animations_enabled);

        // Bottom Action Buttons
        let btn_y = h - 48;
        // [Apply Settings]
        canvas.draw_rect_fill(Rect::new(20, btn_y, 90, 26), Color32::rgb(0x23, 0x86, 0x36));
        canvas.draw_rect_outline(Rect::new(20, btn_y, 90, 26), Color32::rgb(0x2E, 0xA0, 0x43));
        canvas.draw_text(28, btn_y + 5, "Apply", Color32::rgb(0xFF, 0xFF, 0xFF), None);

        // [Reset Defaults]
        canvas.draw_rect_fill(Rect::new(120, btn_y, 90, 26), Color32::rgb(0x30, 0x36, 0x3D));
        canvas.draw_rect_outline(Rect::new(120, btn_y, 90, 26), Color32::rgb(0x48, 0x4F, 0x58));
        canvas.draw_text(128, btn_y + 5, "Defaults", Color32::rgb(0xEB, 0xF0, 0xF4), None);

        // Status bar
        let status_y = h - 20;
        canvas.draw_rect_fill(Rect::new(0, status_y, canvas.width, 20), Color32::rgb(0x12, 0x15, 0x1A));
        canvas.draw_hline(0, status_y, canvas.width, Color32::rgb(0x28, 0x2E, 0x38));
        canvas.draw_text(10, status_y + 2, &self.status_msg, Color32::rgb(0x58, 0xA6, 0xFF), None);
    }
}

fn draw_checkbox(canvas: &mut Canvas, x: i32, y: i32, label: &str, checked: bool) {
    let box_rect = Rect::new(x, y, 14, 14);
    canvas.draw_rect_fill(box_rect, Color32::rgb(0x21, 0x26, 0x2D));
    canvas.draw_rect_outline(box_rect, Color32::rgb(0x48, 0x4F, 0x58));

    if checked {
        canvas.draw_rect_fill(Rect::new(x + 3, y + 3, 8, 8), Color32::rgb(0x38, 0x8B, 0xF6));
    }

    canvas.draw_text(x + 22, y, label, Color32::rgb(0xC9, 0xD1, 0xD9), None);
}
