//! Integrate OS — Text Editor Application
//!
//! Provides a multi-line code and document editor supporting file load/save,
//! system clipboard integration (copy/paste), cursor navigation, line numbers,
//! and dirty state tracking.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use crate::framework::app::Application;
use crate::framework::clipboard;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::fs;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};

/// Multi-line Text Editor Application.
pub struct TextEditorApp {
    pub file_path: String,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub is_dirty: bool,
    pub status_msg: String,
}

impl TextEditorApp {
    pub fn new() -> Self {
        Self {
            file_path: String::from("/notes.txt"),
            lines: vec![
                String::from("// Welcome to Integrate OS Text Editor"),
                String::from("fn main() {"),
                String::from("    println!(\"Hello from Integrate OS!\");"),
                String::from("}"),
            ],
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            is_dirty: false,
            status_msg: String::from("Ready"),
        }
    }

    /// Loads a text file from VFS into the editor buffer.
    pub fn load_file(&mut self, path: &str) -> Result<(), &'static str> {
        let fd = fs::open(path, fs::OpenFlags::READ)?;
        let mut buf = [0u8; 4096];
        let bytes_read = fs::read(fd, &mut buf)?;
        let _ = fs::close(fd);

        let content = core::str::from_utf8(&buf[..bytes_read])
            .map_err(|_| "File contains invalid UTF-8")?;

        self.lines.clear();
        for line in content.lines() {
            self.lines.push(String::from(line));
        }
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        self.file_path = String::from(path);
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_row = 0;
        self.is_dirty = false;
        self.status_msg = String::from("File loaded");
        Ok(())
    }

    /// Saves current editor buffer into the VFS file.
    pub fn save_file(&mut self) -> Result<(), &'static str> {
        let flags = fs::OpenFlags::WRITE | fs::OpenFlags::CREATE | fs::OpenFlags::TRUNCATE;
        let fd = fs::open(&self.file_path, flags)?;

        let mut content = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            content.extend_from_slice(line.as_bytes());
            if i + 1 < self.lines.len() {
                content.push(b'\n');
            }
        }

        let res = fs::write(fd, &content);
        let _ = fs::close(fd);

        res.map(|_| {
            self.is_dirty = false;
            self.status_msg = String::from("File saved successfully");
        })
    }

    /// Copies the current line into the system clipboard.
    pub fn copy_current_line(&mut self) {
        if self.cursor_row < self.lines.len() {
            clipboard::set(&self.lines[self.cursor_row]);
            self.status_msg = String::from("Copied line to clipboard");
        }
    }

    /// Pastes text from the system clipboard at the cursor position.
    pub fn paste_clipboard(&mut self) {
        if let Some(text) = clipboard::get() {
            if self.cursor_row >= self.lines.len() {
                self.lines.push(String::new());
            }

            let cur_line = &mut self.lines[self.cursor_row];
            self.cursor_col = self.cursor_col.min(cur_line.len());

            cur_line.insert_str(self.cursor_col, &text);
            self.cursor_col += text.len();
            self.is_dirty = true;
            self.status_msg = String::from("Pasted from clipboard");
        }
    }

    /// Inserts a character into the current line at cursor position.
    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
        }

        let cur_line = &mut self.lines[self.cursor_row];
        self.cursor_col = self.cursor_col.min(cur_line.len());
        cur_line.insert(self.cursor_col, ch);
        self.cursor_col += 1;
        self.is_dirty = true;
    }

    /// Handles Enter key: splits line at cursor into two lines.
    pub fn handle_enter(&mut self) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
        }

        let cur_line = self.lines[self.cursor_row].clone();
        self.cursor_col = self.cursor_col.min(cur_line.len());

        let left = String::from(&cur_line[..self.cursor_col]);
        let right = String::from(&cur_line[self.cursor_col..]);

        self.lines[self.cursor_row] = left;
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, right);
        self.cursor_col = 0;
        self.is_dirty = true;
    }

    /// Handles Backspace key: deletes preceding character or joins lines.
    pub fn handle_backspace(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }

        if self.cursor_col > 0 {
            let cur_line = &mut self.lines[self.cursor_row];
            if self.cursor_col <= cur_line.len() {
                self.cursor_col -= 1;
                cur_line.remove(self.cursor_col);
                self.is_dirty = true;
            }
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            let prev_len = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.cursor_col = prev_len;
            self.is_dirty = true;
        }
    }
}

impl Application for TextEditorApp {
    fn name(&self) -> &str {
        "Text Editor"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Input(ref we) = *event {
            match *we {
                WidgetEvent::KeyDown { code, character, modifiers } => {
                    if modifiers.is_ctrl() {
                        match code {
                            KeyCode::C => {
                                self.copy_current_line();
                                env.request_redraw();
                                return;
                            }
                            KeyCode::V => {
                                self.paste_clipboard();
                                env.request_redraw();
                                return;
                            }
                            KeyCode::S => {
                                let _ = self.save_file();
                                env.request_redraw();
                                return;
                            }
                            _ => {}
                        }
                    }

                    match code {
                        KeyCode::Enter => {
                            self.handle_enter();
                            env.request_redraw();
                        }
                        KeyCode::Backspace => {
                            self.handle_backspace();
                            env.request_redraw();
                        }
                        KeyCode::Up => {
                            if self.cursor_row > 0 {
                                self.cursor_row -= 1;
                                if self.cursor_row < self.scroll_row {
                                    self.scroll_row = self.cursor_row;
                                }
                                self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                                env.request_redraw();
                            }
                        }
                        KeyCode::Down => {
                            if self.cursor_row + 1 < self.lines.len() {
                                self.cursor_row += 1;
                                self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                                env.request_redraw();
                            }
                        }
                        KeyCode::Left => {
                            self.cursor_col = self.cursor_col.saturating_sub(1);
                            env.request_redraw();
                        }
                        KeyCode::Right => {
                            if self.cursor_row < self.lines.len() {
                                self.cursor_col = (self.cursor_col + 1).min(self.lines[self.cursor_row].len());
                                env.request_redraw();
                            }
                        }
                        KeyCode::Home => {
                            self.cursor_col = 0;
                            env.request_redraw();
                        }
                        KeyCode::End => {
                            if self.cursor_row < self.lines.len() {
                                self.cursor_col = self.lines[self.cursor_row].len();
                                env.request_redraw();
                            }
                        }
                        _ => {
                            if let Some(ch) = character {
                                if !ch.is_control() {
                                    self.insert_char(ch);
                                    env.request_redraw();
                                }
                            }
                        }
                    }
                }
                WidgetEvent::MouseDown { x, y, button } => {
                    if button == MouseButton::Left {
                        // Toolbar clicks (height: 38px)
                        if y >= 6 && y <= 32 {
                            // [New]: 10..55
                            if x >= 10 && x <= 55 {
                                self.lines = vec![String::new()];
                                self.cursor_row = 0;
                                self.cursor_col = 0;
                                self.is_dirty = false;
                                self.status_msg = String::from("New document");
                                env.request_redraw();
                                return;
                            }
                            // [Open]: 60..110
                            if x >= 60 && x <= 110 {
                                let _ = self.load_file("/notes.txt");
                                env.request_redraw();
                                return;
                            }
                            // [Save]: 115..165
                            if x >= 115 && x <= 165 {
                                let _ = self.save_file();
                                env.request_redraw();
                                return;
                            }
                            // [Copy]: 170..220
                            if x >= 170 && x <= 220 {
                                self.copy_current_line();
                                env.request_redraw();
                                return;
                            }
                            // [Paste]: 225..280
                            if x >= 225 && x <= 280 {
                                self.paste_clipboard();
                                env.request_redraw();
                                return;
                            }
                        }

                        // Text Area click
                        if y >= 40 && y < (env.ui_root.bounds.height as i32 - 24) {
                            let line_idx = self.scroll_row + ((y - 40) / (FONT_HEIGHT as i32)) as usize;
                            if line_idx < self.lines.len() {
                                self.cursor_row = line_idx;
                                let col = ((x - 48).max(0) / (FONT_WIDTH as i32)) as usize;
                                self.cursor_col = col.min(self.lines[line_idx].len());
                                env.request_redraw();
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
        let bg_color = Color32::rgb(0x12, 0x14, 0x18);
        canvas.clear(bg_color);

        let h = canvas.height as i32;

        // 1. Toolbar Bar (y: 0..38)
        let bar_bg = Color32::rgb(0x1F, 0x24, 0x2D);
        canvas.draw_rect_fill(Rect::new(0, 0, canvas.width, 38), bar_bg);
        canvas.draw_hline(0, 38, canvas.width, Color32::rgb(0x2D, 0x33, 0x3F));

        let btn_bg = Color32::rgb(0x2D, 0x33, 0x3F);
        let btn_txt = Color32::rgb(0xEB, 0xF0, 0xF4);

        // Buttons: [New], [Open], [Save], [Copy], [Paste]
        draw_button(canvas, 10, 6, 45, 26, "New", btn_bg, btn_txt);
        draw_button(canvas, 60, 6, 50, 26, "Open", btn_bg, btn_txt);
        draw_button(canvas, 115, 6, 50, 26, "Save", btn_bg, btn_txt);
        draw_button(canvas, 170, 6, 50, 26, "Copy", btn_bg, btn_txt);
        draw_button(canvas, 225, 6, 55, 26, "Paste", btn_bg, btn_txt);

        // File Path Title
        let mut title = String::from("File: ");
        title.push_str(&self.file_path);
        if self.is_dirty {
            title.push_str(" *");
        }
        canvas.draw_text(295, 11, &title, Color32::rgb(0x58, 0xA6, 0xFF), None);

        // 2. Line Gutter & Text Area
        let gutter_w = 44;
        let gutter_bg = Color32::rgb(0x16, 0x18, 0x1D);
        let text_y_start = 42;
        let line_h = FONT_HEIGHT as i32;
        let visible_lines = ((h - 68) / line_h).max(1) as usize;

        // Draw gutter background
        canvas.draw_rect_fill(Rect::new(0, text_y_start, gutter_w, (h - 68) as u32), gutter_bg);
        canvas.draw_vline(gutter_w as i32, text_y_start, (h - 68) as u32, Color32::rgb(0x2D, 0x33, 0x3F));

        for (i, line) in self.lines.iter().skip(self.scroll_row).take(visible_lines).enumerate() {
            let row_idx = self.scroll_row + i;
            let y = text_y_start + (i as i32) * line_h;

            // Highlight active line
            if row_idx == self.cursor_row {
                canvas.draw_rect_fill(
                    Rect::new(gutter_w as i32 + 1, y, canvas.width - gutter_w - 1, line_h as u32),
                    Color32::rgb(0x1A, 0x1E, 0x26),
                );
            }

            // Gutter line number
            let mut num_str = String::new();
            format_line_num(&mut num_str, (row_idx + 1) as u32);
            canvas.draw_text(6, y, &num_str, Color32::rgb(0x48, 0x4F, 0x58), None);

            // Line Text
            let text_color = if line.starts_with("//") {
                Color32::rgb(0x8B, 0x94, 0x9E) // Comment green/gray
            } else if line.contains("fn ") || line.contains("let ") {
                Color32::rgb(0xFF, 0x7B, 0x72) // Keyword coral
            } else {
                Color32::rgb(0xEB, 0xF0, 0xF4) // White
            };
            canvas.draw_text((gutter_w + 6) as i32, y, line, text_color, None);

            // Draw cursor if on this line
            if row_idx == self.cursor_row {
                let cursor_x = (gutter_w + 6) as i32 + (self.cursor_col as i32) * (FONT_WIDTH as i32);
                canvas.draw_rect_fill(
                    Rect::new(cursor_x, y, 2, FONT_HEIGHT as u32),
                    Color32::rgb(0x58, 0xA6, 0xFF),
                );
            }
        }

        // 3. Status Bar (bottom)
        let status_y = h - 22;
        canvas.draw_rect_fill(Rect::new(0, status_y, canvas.width, 22), Color32::rgb(0x16, 0x18, 0x1E));
        canvas.draw_hline(0, status_y, canvas.width, Color32::rgb(0x2D, 0x33, 0x3F));

        let mut status = String::from("Ln ");
        format_line_num(&mut status, (self.cursor_row + 1) as u32);
        status.push_str(", Col ");
        format_line_num(&mut status, (self.cursor_col + 1) as u32);
        status.push_str(" | Lines: ");
        format_line_num(&mut status, self.lines.len() as u32);
        status.push_str(" | ");
        status.push_str(if self.is_dirty { "[Modified] " } else { "[Saved] " });
        status.push_str(&self.status_msg);

        canvas.draw_text(10, status_y + 3, &status, Color32::rgb(0x8B, 0x94, 0x9E), None);
    }
}

fn draw_button(canvas: &mut Canvas, x: i32, y: i32, w: u32, h: u32, text: &str, bg: Color32, fg: Color32) {
    canvas.draw_rect_fill(Rect::new(x, y, w, h), bg);
    canvas.draw_rect_outline(Rect::new(x, y, w, h), Color32::rgb(0x48, 0x4F, 0x58));
    let text_x = x + ((w as i32 - (text.len() as i32 * 8)) / 2).max(4);
    canvas.draw_text(text_x, y + 5, text, fg, None);
}

fn format_line_num(s: &mut String, val: u32) {
    if val == 0 {
        s.push('0');
        return;
    }
    let mut digits = [0u8; 10];
    let mut len = 0;
    let mut temp = val;
    while temp > 0 {
        digits[len] = b'0' + (temp % 10) as u8;
        temp /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        s.push(digits[i] as char);
    }
}
