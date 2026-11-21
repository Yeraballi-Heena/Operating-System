//! Integrate OS — Standard GUI Widgets
//!
//! Provides the core collection of user interface widgets:
//! - `Label`: Text rendering and alignment
//! - `Button`: Interactive buttons with visual styles and click actions
//! - `CheckBox`: Toggleable check boxes
//! - `RadioButton`: Mutually exclusive radio selection
//! - `ProgressBar`: Visual progress indicator
//! - `Slider`: Continuous/stepped value slider with thumb handle
//! - `TextInput`: Single-line editable text input

pub mod button;
pub mod checkbox;
pub mod label;
pub mod progress;
pub mod radio;
pub mod slider;
pub mod text_input;

pub use button::Button;
pub use checkbox::CheckBox;
pub use label::Label;
pub use progress::ProgressBar;
pub use radio::RadioButton;
pub use slider::Slider;
pub use text_input::TextInput;
