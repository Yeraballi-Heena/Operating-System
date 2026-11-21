//! Integrate OS — Application Execution Environment
//!
//! Provides the runtime execution context, UI root container, window linkage,
//! and system service hooks for an active application instance.

use alloc::string::String;
use crate::gui::container::Container;
use crate::gui::types::{EventResponse, WidgetEvent, WidgetId};
use crate::gui::widget::Widget;
use crate::framework::types::InstanceId;
use crate::wm::types::WindowId;

pub struct AppEnv {
    pub instance_id: InstanceId,
    pub window_id: WindowId,
    pub pid: u32,
    pub title: String,
    pub ui_root: Container,
    pub quit_requested: bool,
    pub redraw_requested: bool,
}

impl AppEnv {
    pub fn new(instance_id: InstanceId, window_id: WindowId, pid: u32, width: u32, height: u32) -> Self {
        let mut ui_root = Container::new(0, 0, width, height);
        // Default transparent/clean panel background
        ui_root.bg_color = None;
        ui_root.border_color = None;

        Self {
            instance_id,
            window_id,
            pid,
            title: String::new(),
            ui_root,
            quit_requested: false,
            redraw_requested: true,
        }
    }

    pub fn add_widget<W: Widget + 'static>(&mut self, widget: W) -> WidgetId {
        let id = self.ui_root.add_child(widget);
        self.redraw_requested = true;
        id
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = String::from(title);
    }

    /// Dispatches a widget event into the application's root UI container.
    pub fn dispatch_widget_event(&mut self, event: &WidgetEvent) -> EventResponse {
        let res = self.ui_root.handle_event(event);
        if res.is_consumed() {
            self.redraw_requested = true;
        }
        res
    }
}
