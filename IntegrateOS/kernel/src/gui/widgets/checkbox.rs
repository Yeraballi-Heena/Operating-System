//! Integrate OS — CheckBox Widget
//!
//! Provides a toggleable checkbox with an optional label, custom accent checkmark,
//! and keyboard toggle support.

use alloc::string::String;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::input::events::{KeyCode, MouseButton};

pub struct CheckBox {
    pub id: WidgetId,
    pub bounds: Rect,
    pub text: String,
    pub checked: bool,
    pub action_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
}

impl CheckBox {
    pub fn new(text: &str, x: i32, y: i32, width: u32, height: u32, action_id: u32) -> Self {
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(CHECKBOX_SIZE.max(FONT_H))),
            text: String::from(text),
            checked: false,
            action_id,
            visible: true,
            enabled: true,
            focused: false,
        }
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }
}

impl Widget for CheckBox {
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
                    self.checked = !self.checked;
                    EventResponse::Action(self.action_id)
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { code, .. } => {
                if self.focused && code == KeyCode::Space {
                    self.checked = !self.checked;
                    EventResponse::Action(self.action_id)
                } else {
                    EventResponse::Ignored
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

        let box_y = self.bounds.y + (self.bounds.height.saturating_sub(CHECKBOX_SIZE) / 2) as i32;
        let box_rect = Rect::new(self.bounds.x, box_y, CHECKBOX_SIZE, CHECKBOX_SIZE);

        if self.checked {
            canvas.draw_filled_rect(box_rect, Color32::from_u32(ACCENT_PRIMARY));
            // Checkmark lines
            let bx = box_rect.x;
            let by = box_rect.y;
            let white = Color32::from_u32(TEXT_PRIMARY);
            // Draw checkmark path: (bx+3, by+8) -> (bx+6, by+12) -> (bx+12, by+4)
            canvas.draw_line(bx + 3, by + 8, bx + 6, by + 12, white);
            canvas.draw_line(bx + 4, by + 8, bx + 6, by + 11, white);
            canvas.draw_line(bx + 6, by + 12, bx + 12, by + 4, white);
            canvas.draw_line(bx + 6, by + 11, bx + 11, by + 4, white);
        } else {
            canvas.draw_filled_rect(box_rect, Color32::from_u32(BG_INPUT));
        }

        let border_color = if self.focused {
            BORDER_FOCUSED
        } else {
            BORDER_DEFAULT
        };
        canvas.draw_rect(box_rect, Color32::from_u32(border_color));

        // Draw label text beside the box
        if !self.text.is_empty() {
            let text_x = self.bounds.x + (CHECKBOX_SIZE as i32) + 8;
            let text_y = self.bounds.y + (self.bounds.height.saturating_sub(FONT_H) / 2) as i32;
            let fg = if !self.enabled {
                TEXT_DISABLED
            } else {
                TEXT_PRIMARY
            };
            canvas.draw_text(text_x, text_y, &self.text, Color32::from_u32(fg), None);
        }
    }
}
