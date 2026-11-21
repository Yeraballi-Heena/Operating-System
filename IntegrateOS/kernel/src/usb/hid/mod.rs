//! Integrate OS — USB Human Interface Device (HID) Subsystem
//!
//! Provides USB Boot Keyboard and Mouse drivers with automatic OS input event dispatch.

pub mod keyboard;
pub mod mouse;
pub mod usage;

pub use keyboard::UsbHidKeyboard;
pub use mouse::UsbHidMouse;
pub use usage::{hid_usage_to_ascii, hid_usage_to_keycode};
