//! Integrate OS — RadioButton Widget
//!
//! Provides mutually-exclusive radio buttons with group identifiers,
//! custom dot indicators, and keyboard selection.

use alloc::string::String;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::input::events::{KeyCode, MouseButton};

pub struct RadioButton {
    pub id: WidgetId,
    pub bounds: Rect,
    pub group_id: u32,
    pub text: String,
    pub selected: bool,
    pub action_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
}

impl RadioButton {
    pub fn new(
        text: &str,
        group_id: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        action_id: u32,
    ) -> Self {
        let min_h = ((RADIO_RADIUS * 2) as u32).max(FONT_H);
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(min_h)),
            group_id,
            text: String::from(text),
            selected: false,
            action_id,
            visible: true,
            enabled: true,
            focused: false,
        }
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Widget for RadioButton {
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
                    if !self.selected {
                        self.selected = true;
                        EventResponse::Action(self.action_id)
                    } else {
                        EventResponse::Consumed
                    }
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { code, .. } => {
                if self.focused && (code == KeyCode::Space || code == KeyCode::Enter) {
                    if !self.selected {
                        self.selected = true;
                        EventResponse::Action(self.action_id)
                    } else {
                        EventResponse::Consumed
                    }
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

        let cx = self.bounds.x + RADIO_RADIUS + 2;
        let cy = self.bounds.y + (self.bounds.height as i32) / 2;

        if self.selected {
            canvas.draw_filled_circle(cx, cy, RADIO_RADIUS, Color32::from_u32(ACCENT_PRIMARY));
            // Inner white dot
            canvas.draw_filled_circle(cx, cy, 3, Color32::from_u32(TEXT_PRIMARY));
        } else {
            canvas.draw_filled_circle(cx, cy, RADIO_RADIUS, Color32::from_u32(BG_INPUT));
            let border_col = if self.focused {
                BORDER_FOCUSED
            } else {
                BORDER_DEFAULT
            };
            canvas.draw_circle(cx, cy, RADIO_RADIUS, Color32::from_u32(border_col));
        }

        if !self.text.is_empty() {
            let text_x = self.bounds.x + (RADIO_RADIUS * 2 + 8) as i32;
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
