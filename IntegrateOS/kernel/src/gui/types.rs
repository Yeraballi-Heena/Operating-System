//! Integrate OS — GUI Toolkit Core Types & Event Model
//!
//! Provides the fundamental types, identifiers, event descriptions,
//! and interaction responses for the GUI widget subsystem.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::input::events::{KeyCode, Modifiers, MouseButton};

static NEXT_WIDGET_ID: AtomicU32 = AtomicU32::new(1);

/// Strongly-typed monotonic identifier for GUI widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetId(pub u32);

impl WidgetId {
    /// Generates a unique monotonic WidgetId.
    pub fn next() -> Self {
        Self(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Interactive visual state of a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// Layout flow orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Alignment within layout cells or bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

/// Visual style archetype for buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
    Success,
}

/// High-level GUI widget input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetEvent {
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseDown {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    MouseUp {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    KeyDown {
        code: KeyCode,
        modifiers: Modifiers,
        character: Option<char>,
    },
    KeyUp {
        code: KeyCode,
        modifiers: Modifiers,
    },
    FocusGained,
    FocusLost,
}

/// Result of dispatching an event to a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResponse {
    /// The event was not handled or ignored by the widget.
    Ignored,
    /// The event was consumed by the widget (e.g. state updated), no action triggered.
    Consumed,
    /// The event was consumed and triggered an interactive user action / callback ID.
    Action(u32),
}

impl EventResponse {
    pub const fn is_consumed(&self) -> bool {
        !matches!(self, EventResponse::Ignored)
    }

    pub const fn is_action(&self) -> bool {
        matches!(self, EventResponse::Action(_))
    }
}
