//! Integrate OS — Text Input Widget
//!
//! Provides single-line interactive text editing with cursor positioning,
//! backspace/delete, navigation keys, and focus management.

use alloc::string::String;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::input::events::{KeyCode, MouseButton};

pub struct TextInput {
    pub id: WidgetId,
    pub bounds: Rect,
    pub text: String,
    pub placeholder: String,
    pub cursor_pos: usize,
    pub max_len: usize,
    pub action_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
}

impl TextInput {
    pub fn new(
        placeholder: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        action_id: u32,
    ) -> Self {
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(FONT_H + 8)),
            text: String::new(),
            placeholder: String::from(placeholder),
            cursor_pos: 0,
            max_len: 128,
            action_id,
            visible: true,
            enabled: true,
            focused: false,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.set_text(text);
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        let limit = text.len().min(self.max_len);
        self.text.push_str(&text[..limit]);
        self.cursor_pos = self.text.len();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_pos = 0;
    }
}

impl Widget for TextInput {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.focused = false;
        }
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResponse {
        if !self.visible || !self.enabled {
            return EventResponse::Ignored;
        }

        match *event {
            WidgetEvent::MouseDown { x, y, button } => {
                if button == MouseButton::Left && self.bounds.contains(x, y) {
                    self.focused = true;
                    // Calculate cursor position from click X
                    let rel_x = (x - (self.bounds.x + 6)).max(0) as u32;
                    let char_idx = (rel_x / FONT_W) as usize;
                    self.cursor_pos = char_idx.min(self.text.len());
                    EventResponse::Consumed
                } else if self.focused {
                    self.focused = false;
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { code, character, .. } => {
                if !self.focused {
                    return EventResponse::Ignored;
                }

                match code {
                    KeyCode::Backspace => {
                        if self.cursor_pos > 0 && !self.text.is_empty() {
                            self.cursor_pos -= 1;
                            self.text.remove(self.cursor_pos);
                            EventResponse::Consumed
                        } else {
                            EventResponse::Consumed
                        }
                    }
                    KeyCode::Delete => {
                        if self.cursor_pos < self.text.len() {
                            self.text.remove(self.cursor_pos);
                            EventResponse::Consumed
                        } else {
                            EventResponse::Consumed
                        }
                    }
                    KeyCode::Left => {
                        self.cursor_pos = self.cursor_pos.saturating_sub(1);
                        EventResponse::Consumed
                    }
                    KeyCode::Right => {
                        self.cursor_pos = (self.cursor_pos + 1).min(self.text.len());
                        EventResponse::Consumed
                    }
                    KeyCode::Home => {
                        self.cursor_pos = 0;
                        EventResponse::Consumed
                    }
                    KeyCode::End => {
                        self.cursor_pos = self.text.len();
                        EventResponse::Consumed
                    }
                    KeyCode::Enter => {
                        EventResponse::Action(self.action_id)
                    }
                    _ => {
                        if let Some(ch) = character {
                            if !ch.is_control() && self.text.len() < self.max_len {
                                self.text.insert(self.cursor_pos, ch);
                                self.cursor_pos += 1;
                                EventResponse::Consumed
                            } else {
                                EventResponse::Ignored
                            }
                        } else {
                            EventResponse::Ignored
                        }
                    }
                }
            }
            WidgetEvent::FocusGained => {
                self.focused = true;
                EventResponse::Consumed
            }
            WidgetEvent::FocusLost => {
                self.focused = false;
                EventResponse::Consumed
            }
            _ => EventResponse::Ignored,
        }
    }

    fn render(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        // 1. Draw input box background
        canvas.draw_filled_rect(self.bounds, Color32::from_u32(BG_INPUT));

        // 2. Draw border (accent blue when focused)
        let border_col = if self.focused {
            BORDER_FOCUSED
        } else if !self.enabled {
            BORDER_DISABLED
        } else {
            BORDER_DEFAULT
        };
        canvas.draw_rect(self.bounds, Color32::from_u32(border_col));

        // 3. Draw text or placeholder
        let text_x = self.bounds.x + 6;
        let text_y = self.bounds.y + (self.bounds.height.saturating_sub(FONT_H) / 2) as i32;

        if self.text.is_empty() {
            if !self.focused && !self.placeholder.is_empty() {
                canvas.draw_text(text_x, text_y, &self.placeholder, Color32::from_u32(TEXT_SECONDARY), None);
            }
        } else {
            let fg = if !self.enabled {
                TEXT_DISABLED
            } else {
                TEXT_PRIMARY
            };
            canvas.draw_text(text_x, text_y, &self.text, Color32::from_u32(fg), None);
        }

        // 4. Draw cursor caret if focused
        if self.focused && self.enabled {
            let caret_x = text_x + ((self.cursor_pos as u32) * FONT_W) as i32;
            let caret_h = FONT_H.min(self.bounds.height.saturating_sub(4));
            let caret_y = text_y;
            canvas.draw_vline(caret_x, caret_y, caret_h, Color32::from_u32(ACCENT_PRIMARY));
        }
    }
}
