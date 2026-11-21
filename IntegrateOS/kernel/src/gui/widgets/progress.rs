//! Integrate OS — Progress Bar Widget
//!
//! Visualizes deterministic progress percentages with custom fill accents,
//! background trough, and optional centered text percentage display.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;

pub struct ProgressBar {
    pub id: WidgetId,
    pub bounds: Rect,
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub show_text: bool,
    pub visible: bool,
    pub enabled: bool,
}

impl ProgressBar {
    pub fn new(x: i32, y: i32, width: u32, height: u32, min: i32, max: i32) -> Self {
        let max = max.max(min + 1);
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(PROGRESS_BAR_HEIGHT)),
            min,
            max,
            value: min,
            show_text: true,
            visible: true,
            enabled: true,
        }
    }

    pub fn with_value(mut self, value: i32) -> Self {
        self.set_value(value);
        self
    }

    pub fn with_show_text(mut self, show_text: bool) -> Self {
        self.show_text = show_text;
        self
    }

    pub fn set_value(&mut self, value: i32) {
        self.value = value.clamp(self.min, self.max);
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn percentage(&self) -> u32 {
        let span = (self.max - self.min).max(1) as u64;
        let val = (self.value - self.min).max(0) as u64;
        ((val * 100) / span) as u32
    }
}

impl Widget for ProgressBar {
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

        // 1. Draw trough background
        canvas.draw_filled_rect(self.bounds, Color32::from_u32(PROGRESS_BG));
        canvas.draw_rect(self.bounds, Color32::from_u32(PROGRESS_BORDER));

        // 2. Draw filled portion
        let inner_w = self.bounds.width.saturating_sub(2);
        let inner_h = self.bounds.height.saturating_sub(2);
        let span = (self.max - self.min).max(1) as u64;
        let cur = (self.value - self.min).max(0) as u64;
        let fill_w = ((cur * (inner_w as u64)) / span) as u32;

        if fill_w > 0 && inner_h > 0 {
            let fill_rect = Rect::new(self.bounds.x + 1, self.bounds.y + 1, fill_w, inner_h);
            canvas.draw_filled_rect(fill_rect, Color32::from_u32(PROGRESS_FILL));
        }

        // 3. Draw text percentage e.g. "65%"
        if self.show_text && self.bounds.height >= FONT_H {
            let pct = self.percentage();
            let mut text_buf = [0u8; 8];
            let len;
            if pct >= 100 {
                text_buf[0] = b'1';
                text_buf[1] = b'0';
                text_buf[2] = b'0';
                text_buf[3] = b'%';
                len = 4;
            } else if pct >= 10 {
                text_buf[0] = b'0' + ((pct / 10) as u8);
                text_buf[1] = b'0' + ((pct % 10) as u8);
                text_buf[2] = b'%';
                len = 3;
            } else {
                text_buf[0] = b'0' + (pct as u8);
                text_buf[1] = b'%';
                len = 2;
            }

            if let Ok(pct_str) = core::str::from_utf8(&text_buf[..len]) {
                let text_w = (len as u32) * FONT_W;
                let text_x = self.bounds.x + (self.bounds.width.saturating_sub(text_w) / 2) as i32;
                let text_y = self.bounds.y + (self.bounds.height.saturating_sub(FONT_H) / 2) as i32;
                canvas.draw_text(text_x, text_y, pct_str, Color32::from_u32(TEXT_PRIMARY), None);
            }
        }
    }
}
