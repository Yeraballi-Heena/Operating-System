//! Integrate OS — Graphical File Manager Application
//!
//! Provides an interactive VFS file browser supporting directory listing,
//! hierarchical folder navigation, file selection, metadata inspection, and status reporting.

use alloc::string::String;
use alloc::vec::Vec;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::fs;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};

/// Directory entry item displayed in the file manager.
#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Graphical File Manager Application.
pub struct FileManagerApp {
    pub current_path: String,
    pub entries: Vec<FileItem>,
    pub selected_index: Option<usize>,
    pub scroll_offset: usize,
    pub status_msg: String,
}

impl FileManagerApp {
    pub fn new() -> Self {
        let mut app = Self {
            current_path: String::from("/"),
            entries: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            status_msg: String::from("Ready"),
        };
        app.refresh();
        app
    }

    /// Reloads directory contents from the VFS.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_index = None;
        self.scroll_offset = 0;

        match fs::readdir(&self.current_path) {
            Ok(vfs_entries) => {
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in vfs_entries {
                    let is_dir = entry.node_type == fs::NodeType::Directory;
                    let item = FileItem {
                        name: entry.name,
                        is_dir,
                        size: entry.size,
                    };
                    if is_dir {
                        dirs.push(item);
                    } else {
                        files.push(item);
                    }
                }

                // Sort directories first, then files
                dirs.sort_by(|a, b| a.name.cmp(&b.name));
                files.sort_by(|a, b| a.name.cmp(&b.name));

                self.entries.extend(dirs);
                self.entries.extend(files);

                let mut msg = String::from("Loaded ");
                let count = self.entries.len();
                let mut temp = count;
                let mut digits = [0u8; 10];
                let mut d_len = 0;
                if temp == 0 {
                    digits[0] = b'0';
                    d_len = 1;
                } else {
                    while temp > 0 {
                        digits[d_len] = b'0' + (temp % 10) as u8;
                        temp /= 10;
                        d_len += 1;
                    }
                }
                for i in (0..d_len).rev() {
                    msg.push(digits[i] as char);
                }
                msg.push_str(" items");
                self.status_msg = msg;
            }
            Err(e) => {
                let mut msg = String::from("Error: ");
                msg.push_str(e);
                self.status_msg = msg;
            }
        }
    }

    /// Navigates into a sub-path or absolute path.
    pub fn navigate_to(&mut self, path: &str) {
        if path.starts_with('/') {
            self.current_path = String::from(path);
        } else {
            if !self.current_path.ends_with('/') {
                self.current_path.push('/');
            }
            self.current_path.push_str(path);
        }
        self.refresh();
    }

    /// Navigates up to the parent directory.
    pub fn navigate_up(&mut self) {
        if self.current_path == "/" {
            return;
        }

        let trimmed = self.current_path.trim_end_matches('/');
        if let Some(idx) = trimmed.rfind('/') {
            if idx == 0 {
                self.current_path = String::from("/");
            } else {
                self.current_path = String::from(&trimmed[..idx]);
            }
        } else {
            self.current_path = String::from("/");
        }
        self.refresh();
    }

    /// Opens or activates the currently selected item.
    pub fn open_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.entries.len() {
                let item = self.entries[idx].clone();
                if item.is_dir {
                    let mut next_path = self.current_path.clone();
                    if !next_path.ends_with('/') {
                        next_path.push('/');
                    }
                    next_path.push_str(&item.name);
                    self.navigate_to(&next_path);
                } else {
                    let mut msg = String::from("Selected file: ");
                    msg.push_str(&item.name);
                    self.status_msg = msg;
                }
            }
        }
    }
}

impl Application for FileManagerApp {
    fn name(&self) -> &str {
        "File Manager"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        self.refresh();
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Input(ref we) = *event {
            match *we {
                WidgetEvent::KeyDown { code, .. } => {
                    match code {
                        KeyCode::Up => {
                            if let Some(idx) = self.selected_index {
                                if idx > 0 {
                                    self.selected_index = Some(idx - 1);
                                    if idx - 1 < self.scroll_offset {
                                        self.scroll_offset = idx - 1;
                                    }
                                }
                            } else if !self.entries.is_empty() {
                                self.selected_index = Some(0);
                            }
                            env.request_redraw();
                        }
                        KeyCode::Down => {
                            if let Some(idx) = self.selected_index {
                                if idx + 1 < self.entries.len() {
                                    self.selected_index = Some(idx + 1);
                                }
                            } else if !self.entries.is_empty() {
                                self.selected_index = Some(0);
                            }
                            env.request_redraw();
                        }
                        KeyCode::Enter => {
                            self.open_selected();
                            env.request_redraw();
                        }
                        KeyCode::Backspace => {
                            self.navigate_up();
                            env.request_redraw();
                        }
                        _ => {}
                    }
                }
                WidgetEvent::MouseDown { x, y, button } => {
                    if button == MouseButton::Left {
                        // Toolbar clicks (height: 36px)
                        if y >= 6 && y <= 32 {
                            // Up button: x: 10..55
                            if x >= 10 && x <= 55 {
                                self.navigate_up();
                                env.request_redraw();
                                return;
                            }
                            // Root button: x: 60..115
                            if x >= 60 && x <= 115 {
                                self.navigate_to("/");
                                env.request_redraw();
                                return;
                            }
                            // SSD button: x: 120..220
                            if x >= 120 && x <= 220 {
                                self.navigate_to("/mnt/ssd");
                                env.request_redraw();
                                return;
                            }
                            // Refresh button: x: 226..296
                            if x >= 226 && x <= 296 {
                                self.refresh();
                                env.request_redraw();
                                return;
                            }
                        }

                        // Entry list items: y from 40 down to canvas.height - 24
                        if y >= 40 {
                            let row_h = 22;
                            let clicked_row = (y - 40) / row_h;
                            let item_idx = self.scroll_offset + clicked_row as usize;

                            if item_idx < self.entries.len() {
                                if self.selected_index == Some(item_idx) {
                                    // Second click on selected row activates/opens it
                                    self.open_selected();
                                } else {
                                    self.selected_index = Some(item_idx);
                                    let item = &self.entries[item_idx];
                                    let mut s = String::from(if item.is_dir { "Folder: " } else { "File: " });
                                    s.push_str(&item.name);
                                    self.status_msg = s;
                                }
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
        let bg_color = Color32::rgb(0x18, 0x1B, 0x22);
        canvas.clear(bg_color);

        let h = canvas.height as i32;

        // 1. Top Toolbar Bar (y: 0..38)
        let toolbar_bg = Color32::rgb(0x21, 0x26, 0x2D);
        canvas.draw_rect_fill(Rect::new(0, 0, canvas.width, 38), toolbar_bg);
        canvas.draw_hline(0, 38, canvas.width, Color32::rgb(0x30, 0x36, 0x3D));

        // Toolbar Buttons: [Up], [Root], [SSD: /mnt/ssd], [Refresh]
        let btn_bg = Color32::rgb(0x30, 0x36, 0x3D);
        let btn_text_color = Color32::rgb(0xEB, 0xF0, 0xF4);

        // Up Button
        canvas.draw_rect_fill(Rect::new(10, 6, 45, 26), btn_bg);
        canvas.draw_rect_outline(Rect::new(10, 6, 45, 26), Color32::rgb(0x48, 0x4F, 0x58));
        canvas.draw_text(18, 11, "Up", btn_text_color, None);

        // Root Button
        canvas.draw_rect_fill(Rect::new(60, 6, 55, 26), btn_bg);
        canvas.draw_rect_outline(Rect::new(60, 6, 55, 26), Color32::rgb(0x48, 0x4F, 0x58));
        canvas.draw_text(66, 11, "Root /", btn_text_color, None);

        // SSD: /mnt/ssd Button (Green highlight)
        canvas.draw_rect_fill(Rect::new(120, 6, 100, 26), Color32::rgb(0x16, 0x4E, 0x63));
        canvas.draw_rect_outline(Rect::new(120, 6, 100, 26), Color32::rgb(0x06, 0xB6, 0xD4));
        canvas.draw_text(126, 11, "SSD: /mnt/ssd", Color32::rgb(0x67, 0xE8, 0xF9), None);

        // Refresh Button
        canvas.draw_rect_fill(Rect::new(226, 6, 65, 26), btn_bg);
        canvas.draw_rect_outline(Rect::new(226, 6, 65, 26), Color32::rgb(0x48, 0x4F, 0x58));
        canvas.draw_text(232, 11, "Refresh", btn_text_color, None);

        // Path Display
        let mut path_str = String::from("Path: ");
        path_str.push_str(&self.current_path);
        canvas.draw_text(210, 11, &path_str, Color32::rgb(0x58, 0xA6, 0xFF), None);

        // 2. Directory Entries List
        let list_y_start = 42;
        let row_h = 22;
        let visible_rows = ((h - 70) / row_h).max(1) as usize;

        for (i, entry) in self.entries.iter().skip(self.scroll_offset).take(visible_rows).enumerate() {
            let row_idx = self.scroll_offset + i;
            let y = list_y_start + (i as i32) * row_h;
            let is_selected = self.selected_index == Some(row_idx);

            // Row background highlight
            if is_selected {
                canvas.draw_rect_fill(Rect::new(4, y, canvas.width - 8, row_h as u32), Color32::rgb(0x26, 0x38, 0x59));
                canvas.draw_rect_outline(Rect::new(4, y, canvas.width - 8, row_h as u32), Color32::rgb(0x38, 0x8B, 0xF6));
            } else if i % 2 == 1 {
                canvas.draw_rect_fill(Rect::new(4, y, canvas.width - 8, row_h as u32), Color32::rgb(0x1B, 0x1F, 0x27));
            }

            // Type icon & text
            let (type_tag, tag_color) = if entry.is_dir {
                ("[DIR] ", Color32::rgb(0xF6, 0xC1, 0x77))
            } else {
                ("[FILE]", Color32::rgb(0x8B, 0x94, 0x9E))
            };

            canvas.draw_text(14, y + 3, type_tag, tag_color, None);
            let name_color = if entry.is_dir {
                Color32::rgb(0x7E, 0xC7, 0xFF)
            } else {
                Color32::rgb(0xEB, 0xF0, 0xF4)
            };
            canvas.draw_text(74, y + 3, &entry.name, name_color, None);
        }

        // 3. Status Bar at Bottom
        let status_y = h - 24;
        canvas.draw_rect_fill(Rect::new(0, status_y, canvas.width, 24), Color32::rgb(0x16, 0x1B, 0x22));
        canvas.draw_hline(0, status_y, canvas.width, Color32::rgb(0x30, 0x36, 0x3D));
        canvas.draw_text(10, status_y + 4, &self.status_msg, Color32::rgb(0x8B, 0x94, 0x9E), None);
    }
}
