//! Integrate OS — GUI Toolkit Subsystem
//!
//! Provides the foundational user interface framework:
//! - Widget trait model and monotonic ID tracking (`widget.rs`, `types.rs`)
//! - Visual theming and dark-mode styling (`theme.rs`)
//! - Interactive widgets: Label, Button, CheckBox, RadioButton, ProgressBar, Slider, TextInput (`widgets/`)
//! - Linear box layout engine (`layout.rs`)
//! - Hierarchical panel container with focus & event dispatch (`container.rs`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod container;
pub mod layout;
pub mod theme;
pub mod types;
pub mod widget;
pub mod widgets;
pub mod tests;

pub use container::Container;
pub use layout::BoxLayout;
pub use types::*;
pub use widget::Widget;
pub use widgets::*;

use crate::klog_ok;

/// Initializes the GUI Toolkit subsystem.
pub fn init() {
    klog_ok!("GUI Toolkit Initialized: Core widget library, layout engine & event dispatch active");
}
