//! Integrate OS — Container & Panel Widget
//!
//! Provides hierarchical widget grouping, background styling, automated layout
//! distribution, focus management, and event bubbling.

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::layout::BoxLayout;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;

pub struct Container {
    pub id: WidgetId,
    pub bounds: Rect,
    pub children: Vec<Box<dyn Widget>>,
    pub bg_color: Option<Color32>,
    pub border_color: Option<Color32>,
    pub focused_child: Option<WidgetId>,
    pub visible: bool,
    pub enabled: bool,
}

impl Container {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            id: WidgetId::next(),
            bounds: Rect::new(x, y, width, height),
            children: Vec::new(),
            bg_color: Some(Color32::from_u32(BG_PANEL)),
            border_color: Some(Color32::from_u32(BORDER_DEFAULT)),
            focused_child: None,
            visible: true,
            enabled: true,
        }
    }

    pub fn with_background(mut self, bg: Option<u32>) -> Self {
        self.bg_color = bg.map(Color32::from_u32);
        self
    }

    pub fn with_border(mut self, border: Option<u32>) -> Self {
        self.border_color = border.map(Color32::from_u32);
        self
    }

    pub fn add_child<W: Widget + 'static>(&mut self, child: W) -> WidgetId {
        let id = child.id();
        self.children.push(Box::new(child));
        id
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn child(&self, index: usize) -> Option<&Box<dyn Widget>> {
        self.children.get(index)
    }

    pub fn child_mut(&mut self, index: usize) -> Option<&mut Box<dyn Widget>> {
        self.children.get_mut(index)
    }

    /// Automatically lays out children using the provided BoxLayout.
    pub fn apply_box_layout(&mut self, layout: &BoxLayout) {
        let item_sizes: Vec<(u32, u32)> = self
            .children
            .iter()
            .map(|c| (c.bounds().width, c.bounds().height))
            .collect();

        let bounds_list = layout.compute_bounds(self.bounds, &item_sizes);
        for (i, b) in bounds_list.into_iter().enumerate() {
            if let Some(child) = self.children.get_mut(i) {
                child.set_bounds(b);
            }
        }
    }
}

impl Widget for Container {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        let dx = bounds.x - self.bounds.x;
        let dy = bounds.y - self.bounds.y;
        self.bounds = bounds;

        // Shift children bounds accordingly
        for child in &mut self.children {
            let cb = child.bounds();
            child.set_bounds(Rect::new(cb.x + dx, cb.y + dy, cb.width, cb.height));
        }
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
        for child in &mut self.children {
            child.set_enabled(enabled);
        }
    }

    fn is_focused(&self) -> bool {
        self.focused_child.is_some()
    }

    fn set_focused(&mut self, focused: bool) {
        if !focused {
            if let Some(focused_id) = self.focused_child.take() {
                for child in &mut self.children {
                    if child.id() == focused_id {
                        child.set_focused(false);
                    }
                }
            }
        }
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResponse {
        if !self.visible || !self.enabled {
            return EventResponse::Ignored;
        }

        match *event {
            WidgetEvent::MouseDown { x, y, button: _ } => {
                if !self.bounds.contains(x, y) {
                    return EventResponse::Ignored;
                }

                // Check children in reverse order (topmost first)
                let mut clicked_child_id = None;
                let mut response = EventResponse::Ignored;

                for child in self.children.iter_mut().rev() {
                    if child.is_visible() && child.bounds().contains(x, y) {
                        let res = child.handle_event(event);
                        if res.is_consumed() {
                            clicked_child_id = Some(child.id());
                            response = res;
                            break;
                        }
                    }
                }

                if let Some(new_focused_id) = clicked_child_id {
                    if self.focused_child != Some(new_focused_id) {
                        if let Some(old_id) = self.focused_child {
                            for child in &mut self.children {
                                if child.id() == old_id {
                                    child.set_focused(false);
                                }
                            }
                        }
                        self.focused_child = Some(new_focused_id);
                        for child in &mut self.children {
                            if child.id() == new_focused_id {
                                child.set_focused(true);
                            }
                        }
                    }
                    return response;
                }

                // Clicked inside container background
                if self.focused_child.is_some() {
                    if let Some(old_id) = self.focused_child.take() {
                        for child in &mut self.children {
                            if child.id() == old_id {
                                child.set_focused(false);
                            }
                        }
                    }
                }
                EventResponse::Consumed
            }
            WidgetEvent::MouseMove { x, y } => {
                let mut any_consumed = false;
                for child in &mut self.children {
                    if child.is_visible() {
                        let res = child.handle_event(event);
                        if res.is_consumed() {
                            any_consumed = true;
                        }
                    }
                }
                if any_consumed {
                    EventResponse::Consumed
                } else if self.bounds.contains(x, y) {
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::MouseUp { x, y, button: _ } => {
                for child in self.children.iter_mut().rev() {
                    if child.is_visible() {
                        let res = child.handle_event(event);
                        if res.is_consumed() {
                            return res;
                        }
                    }
                }
                if self.bounds.contains(x, y) {
                    EventResponse::Consumed
                } else {
                    EventResponse::Ignored
                }
            }
            WidgetEvent::KeyDown { .. } | WidgetEvent::KeyUp { .. } => {
                if let Some(focused_id) = self.focused_child {
                    for child in &mut self.children {
                        if child.id() == focused_id {
                            return child.handle_event(event);
                        }
                    }
                }
                EventResponse::Ignored
            }
            WidgetEvent::FocusLost => {
                self.set_focused(false);
                EventResponse::Consumed
            }
            WidgetEvent::FocusGained => {
                EventResponse::Consumed
            }
        }
    }

    fn render(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        // Draw background if configured
        if let Some(bg) = self.bg_color {
            canvas.draw_filled_rect(self.bounds, bg);
        }

        // Draw border if configured
        if let Some(border) = self.border_color {
            canvas.draw_rect(self.bounds, border);
        }

        // Render children
        for child in &self.children {
            if child.is_visible() {
                child.render(canvas);
            }
        }
    }
}
