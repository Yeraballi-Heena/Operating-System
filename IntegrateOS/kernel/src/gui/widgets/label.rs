//! Integrate OS — Text Label Widget
//!
//! Renders static or dynamic text strings with configurable color, background,
//! and horizontal alignment.

use alloc::string::String;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::{FONT_H, FONT_W, TEXT_PRIMARY};
use crate::gui::types::{Alignment, EventResponse, WidgetEvent, WidgetId};
use crate::gui::widget::Widget;

pub struct Label {
    pub id: WidgetId,
    pub bounds: Rect,
    pub text: String,
    pub fg_color: Color32,
    pub bg_color: Option<Color32>,
    pub alignment: Alignment,
    pub visible: bool,
    pub enabled: bool,
}

impl Label {
    pub fn new(text: &str, x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(FONT_H)),
            text: String::from(text),
            fg_color: Color32::from_u32(TEXT_PRIMARY),
            bg_color: None,
            alignment: Alignment::Start,
            visible: true,
            enabled: true,
        }
    }

    pub fn with_colors(mut self, fg: u32, bg: Option<u32>) -> Self {
        self.fg_color = Color32::from_u32(fg);
        self.bg_color = bg.map(Color32::from_u32);
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = String::from(text);
    }
}

impl Widget for Label {
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
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn handle_event(&mut self, _event: &WidgetEvent) -> EventResponse {
        EventResponse::Ignored
    }

    fn render(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        if let Some(bg) = self.bg_color {
            canvas.draw_filled_rect(self.bounds, bg);
        }

        let text_w = (self.text.len() as u32) * FONT_W;
        let text_x = match self.alignment {
            Alignment::Start => self.bounds.x,
            Alignment::Center => {
                let avail = self.bounds.width.saturating_sub(text_w);
                self.bounds.x + (avail / 2) as i32
            }
            Alignment::End => {
                let avail = self.bounds.width.saturating_sub(text_w);
                self.bounds.x + avail as i32
            }
            Alignment::Stretch => self.bounds.x,
        };

        let avail_h = self.bounds.height.saturating_sub(FONT_H);
        let text_y = self.bounds.y + (avail_h / 2) as i32;

        canvas.draw_text(text_x, text_y, &self.text, self.fg_color, None);
    }
}
