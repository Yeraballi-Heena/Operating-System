//! Integrate OS — Interactive Slider Widget
//!
//! Provides a continuous or stepped value slider with draggable thumb handle,
//! track fill, and keyboard navigation.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::input::events::{KeyCode, MouseButton};

pub struct Slider {
    pub id: WidgetId,
    pub bounds: Rect,
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub step: i32,
    pub is_dragging: bool,
    pub action_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
}

impl Slider {
    pub fn new(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        min: i32,
        max: i32,
        action_id: u32,
    ) -> Self {
        let max = max.max(min + 1);
        let min_h = ((SLIDER_THUMB_RADIUS * 2 + 4) as u32).max(16);
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height.max(min_h)),
            min,
            max,
            value: min,
            step: 1,
            is_dragging: false,
            action_id,
            visible: true,
            enabled: true,
            focused: false,
        }
    }

    pub fn with_value(mut self, val: i32) -> Self {
        self.set_value(val);
        self
    }

    pub fn with_step(mut self, step: i32) -> Self {
        self.step = step.max(1);
        self
    }

    pub fn set_value(&mut self, val: i32) -> bool {
        let clamped = val.clamp(self.min, self.max);
        let stepped = if self.step > 1 {
            let offset = clamped - self.min;
            let rem = offset % self.step;
            let base = clamped - rem;
            if rem >= self.step / 2 && base + self.step <= self.max {
                base + self.step
            } else {
                base
            }
        } else {
            clamped
        };

        if stepped != self.value {
            self.value = stepped;
            true
        } else {
            false
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    fn value_to_x(&self) -> i32 {
        let track_x = self.bounds.x + (SLIDER_THUMB_RADIUS as i32);
        let track_w = self.bounds.width.saturating_sub((SLIDER_THUMB_RADIUS * 2) as u32);
        if track_w == 0 {
            return track_x;
        }

        let span = (self.max - self.min).max(1) as u64;
        let cur = (self.value - self.min).max(0) as u64;
        track_x + ((cur * (track_w as u64)) / span) as i32
    }

    fn x_to_value(&self, click_x: i32) -> i32 {
        let track_x = self.bounds.x + (SLIDER_THUMB_RADIUS as i32);
        let track_w = self.bounds.width.saturating_sub((SLIDER_THUMB_RADIUS * 2) as u32);
        if track_w == 0 {
            return self.min;
        }

        let rel_x = (click_x - track_x).clamp(0, track_w as i32) as u64;
        let span = (self.max - self.min).max(1) as u64;
        self.min + ((rel_x * span) / (track_w as u64)) as i32
    }
}

impl Widget for Slider {
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
            self.is_dragging = false;
        }
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.is_dragging = false;
        }
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResponse {
        if !self.visible || !self.enabled {
            return EventResponse::Ignored;
        }

        match *event {
            WidgetEvent::MouseDown { x, y, button } => {
                if button == MouseButton::Left && self.bounds.contains(x, y) {
                    self.focused = true;
                    self.is_dragging = true;
                    let new_val = self.x_to_value(x);
                    if self.set_value(new_val) {
                        EventResponse::Action(self.action_id)
                    } else {
                        EventResponse::Consumed
                    }
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::MouseMove { x, .. } => {
                if self.is_dragging {
                    let new_val = self.x_to_value(x);
                    if self.set_value(new_val) {
                        EventResponse::Action(self.action_id)
                    } else {
                        EventResponse::Consumed
                    }
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::MouseUp { button, .. } => {
                if button == MouseButton::Left && self.is_dragging {
                    self.is_dragging = false;
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { code, .. } => {
                if self.focused {
                    match code {
                        KeyCode::Left | KeyCode::Down => {
                            if self.set_value(self.value - self.step) {
                                EventResponse::Action(self.action_id)
                            } else {
                                EventResponse::Consumed
                            }
                        }
                        KeyCode::Right | KeyCode::Up => {
                            if self.set_value(self.value + self.step) {
                                EventResponse::Action(self.action_id)
                            } else {
                                EventResponse::Consumed
                            }
                        }
                        _ => EventResponse::Ignored,
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
                self.is_dragging = false;
                EventResponse::Consumed
            }
            _ => EventResponse::Ignored,
        }
    }

    fn render(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        let track_h = SLIDER_TRACK_HEIGHT;
        let track_y = self.bounds.y + (self.bounds.height.saturating_sub(track_h) / 2) as i32;
        let track_x = self.bounds.x + (SLIDER_THUMB_RADIUS as i32);
        let track_w = self.bounds.width.saturating_sub((SLIDER_THUMB_RADIUS * 2) as u32);

        // 1. Draw inactive background track
        let full_track_rect = Rect::new(track_x, track_y, track_w, track_h);
        canvas.draw_filled_rect(full_track_rect, Color32::from_u32(SLIDER_TRACK_BG));

        // 2. Draw active filled track portion
        let thumb_cx = self.value_to_x();
        let fill_w = (thumb_cx - track_x).max(0) as u32;
        if fill_w > 0 {
            let active_rect = Rect::new(track_x, track_y, fill_w, track_h);
            canvas.draw_filled_rect(active_rect, Color32::from_u32(SLIDER_TRACK_FILL));
        }

        // 3. Draw thumb circle
        let thumb_cy = self.bounds.y + (self.bounds.height as i32) / 2;
        let thumb_bg = if !self.enabled {
            TEXT_DISABLED
        } else if self.is_dragging {
            SLIDER_THUMB_PRESSED
        } else if self.focused {
            SLIDER_THUMB_HOVER
        } else {
            SLIDER_THUMB_BG
        };

        canvas.draw_filled_circle(thumb_cx, thumb_cy, SLIDER_THUMB_RADIUS, Color32::from_u32(thumb_bg));
        canvas.draw_circle(thumb_cx, thumb_cy, SLIDER_THUMB_RADIUS, Color32::from_u32(SLIDER_THUMB_BORDER));
    }
}
