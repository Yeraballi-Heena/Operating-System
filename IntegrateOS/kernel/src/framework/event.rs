//! Integrate OS — Application Framework Event Model
//!
//! Provides the unified event envelope dispatched to running applications.

use crate::gui::types::WidgetEvent;

/// Unified event envelope received by applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    /// Window was resized to new client dimensions.
    WindowResized { width: u32, height: u32 },
    /// Window gained active keyboard/mouse focus.
    WindowFocused,
    /// Window lost focus.
    WindowUnfocused,
    /// Low-level or high-level GUI widget input event.
    Input(WidgetEvent),
    /// Action callback triggered by an interactive UI widget.
    Action(u32),
    /// Periodic timer tick notification with elapsed ticks.
    TimerTick(u64),
    /// Inter-process or internal custom messaging.
    Custom { message_id: u32, payload: u64 },
    /// OS or user requested application termination.
    QuitRequested,
}
