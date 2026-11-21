//! Integrate OS — Interactive Push Button Widget
//!
//! Provides customizable buttons with primary, secondary, danger, and success styles,
//! hover and pressed animations, keyboard activation, and action firing.

use alloc::string::String;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::input::events::{KeyCode, MouseButton};

pub struct Button {
    pub id: WidgetId,
    pub bounds: Rect,
    pub text: String,
    pub style: ButtonStyle,
    pub state: WidgetState,
    pub action_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
}

impl Button {
    pub fn new(text: &str, x: i32, y: i32, width: u32, height: u32, action_id: u32) -> Self {
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(FONT_H + 8)),
            text: String::from(text),
            style: ButtonStyle::Primary,
            state: WidgetState::Normal,
            action_id,
            visible: true,
            enabled: true,
            focused: false,
        }
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn bg_color(&self) -> u32 {
        if !self.enabled {
            return BTN_DISABLED_BG;
        }

        match (self.style, self.state) {
            (ButtonStyle::Primary, WidgetState::Normal) => BTN_PRIMARY_BG,
            (ButtonStyle::Primary, WidgetState::Hover) => BTN_PRIMARY_HOVER,
            (ButtonStyle::Primary, WidgetState::Pressed) => BTN_PRIMARY_PRESSED,

            (ButtonStyle::Secondary, WidgetState::Normal) => BTN_SECONDARY_BG,
            (ButtonStyle::Secondary, WidgetState::Hover) => BTN_SECONDARY_HOVER,
            (ButtonStyle::Secondary, WidgetState::Pressed) => BTN_SECONDARY_PRESSED,

            (ButtonStyle::Danger, WidgetState::Normal) => BTN_DANGER_BG,
            (ButtonStyle::Danger, WidgetState::Hover) => BTN_DANGER_HOVER,
            (ButtonStyle::Danger, WidgetState::Pressed) => BTN_DANGER_PRESSED,

            (ButtonStyle::Success, WidgetState::Normal) => BTN_SUCCESS_BG,
            (ButtonStyle::Success, WidgetState::Hover) => BTN_SUCCESS_HOVER,
            (ButtonStyle::Success, WidgetState::Pressed) => BTN_SUCCESS_PRESSED,

            (_, WidgetState::Disabled) => BTN_DISABLED_BG,
        }
    }
}

impl Widget for Button {
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
            self.state = WidgetState::Disabled;
        } else if self.state == WidgetState::Disabled {
            self.state = WidgetState::Normal;
        }
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused && self.state == WidgetState::Pressed {
            self.state = WidgetState::Normal;
        }
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResponse {
        if !self.visible || !self.enabled {
            return EventResponse::Ignored;
        }

        match *event {
            WidgetEvent::MouseMove { x, y } => {
                let inside = self.bounds.contains(x, y);
                if inside {
                    if self.state == WidgetState::Normal {
                        self.state = WidgetState::Hover;
                        EventResponse::Consumed
                    } else {
                        EventResponse::Ignored
                    }
                } else {
                    if self.state == WidgetState::Hover {
                        self.state = WidgetState::Normal;
                        EventResponse::Consumed
                    } else {
                        EventResponse::Ignored
                    }
                }
            }
            WidgetEvent::MouseDown { x, y, button } => {
                if button == MouseButton::Left && self.bounds.contains(x, y) {
                    self.state = WidgetState::Pressed;
                    self.focused = true;
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::MouseUp { x, y, button } => {
                if button == MouseButton::Left && self.state == WidgetState::Pressed {
                    let inside = self.bounds.contains(x, y);
                    self.state = if inside {
                        WidgetState::Hover
                    } else {
                        WidgetState::Normal
                    };
                    if inside {
                        EventResponse::Action(self.action_id)
                    } else {
                        EventResponse::Consumed
                    }
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { code, .. } => {
                if self.focused && (code == KeyCode::Enter || code == KeyCode::Space) {
                    self.state = WidgetState::Pressed;
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyUp { code, .. } => {
                if self.focused
                    && self.state == WidgetState::Pressed
                    && (code == KeyCode::Enter || code == KeyCode::Space)
                {
                    self.state = WidgetState::Normal;
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
                self.state = WidgetState::Normal;
                EventResponse::Consumed
            }
        }
    }

    fn render(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        let bg = Color32::from_u32(self.bg_color());
        canvas.draw_filled_rect(self.bounds, bg);

        let border_color = if self.focused {
            BORDER_FOCUSED
        } else if self.state == WidgetState::Hover {
            BORDER_HOVER
        } else if !self.enabled {
            BORDER_DISABLED
        } else {
            BORDER_DEFAULT
        };
        canvas.draw_rect(self.bounds, Color32::from_u32(border_color));

        let text_w = (self.text.len() as u32) * FONT_W;
        let text_x = self.bounds.x + (self.bounds.width.saturating_sub(text_w) / 2) as i32;
        let text_y = self.bounds.y + (self.bounds.height.saturating_sub(FONT_H) / 2) as i32;

        let fg = if !self.enabled {
            TEXT_DISABLED
        } else {
            TEXT_PRIMARY
        };
        canvas.draw_text(text_x, text_y, &self.text, Color32::from_u32(fg), None);
    }
}
