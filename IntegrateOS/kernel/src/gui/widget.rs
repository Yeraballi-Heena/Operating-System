//! Integrate OS — Base Widget Trait
//!
//! Defines the polymorphic interface implemented by all user interface elements.

use crate::graphics::canvas::Canvas;
use crate::graphics::rect::Rect;
use crate::gui::types::{EventResponse, WidgetEvent, WidgetId};

/// Core trait implemented by all GUI widgets.
pub trait Widget: Send + Sync {
    /// Returns the unique identifier of the widget.
    fn id(&self) -> WidgetId;

    /// Returns the current bounding rectangle of the widget in parent-relative coordinates.
    fn bounds(&self) -> Rect;

    /// Updates the bounding rectangle of the widget.
    fn set_bounds(&mut self, bounds: Rect);

    /// Checks if the widget is visible.
    fn is_visible(&self) -> bool;

    /// Toggles visibility of the widget.
    fn set_visible(&mut self, visible: bool);

    /// Checks if the widget is enabled for interaction.
    fn is_enabled(&self) -> bool;

    /// Toggles enabled state of the widget.
    fn set_enabled(&mut self, enabled: bool);

    /// Checks if the widget currently has input focus.
    fn is_focused(&self) -> bool;

    /// Sets or clears input focus for the widget.
    fn set_focused(&mut self, focused: bool);

    /// Processes an incoming widget input event.
    fn handle_event(&mut self, event: &WidgetEvent) -> EventResponse;

    /// Renders the widget onto the provided canvas.
    fn render(&self, canvas: &mut Canvas);
}
