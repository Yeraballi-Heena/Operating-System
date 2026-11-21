//! Integrate OS — Application Trait
//!
//! Defines the polymorphic lifecycle interface that all Integrate OS applications implement.

use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;

/// Trait implemented by all graphical and system applications.
pub trait Application: Send + Sync {
    /// Returns the human-readable display name of the application.
    fn name(&self) -> &str;

    /// Invoked immediately after window creation to build UI widgets and allocate state.
    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError>;

    /// Periodic frame update callback for animations, polling, or state progression.
    fn on_update(&mut self, env: &mut AppEnv, delta_ms: u32);

    /// Processes high-level application events, window resizes, and UI actions.
    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent);

    /// Invoked before application destruction and window removal to clean up resources.
    fn on_destroy(&mut self, env: &mut AppEnv);

    /// Custom canvas rendering callback invoked after UI widgets have been rasterized.
    fn render(&self, _canvas: &mut Canvas) {}
}
