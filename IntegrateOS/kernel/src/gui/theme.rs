//! Integrate OS — GUI Toolkit Visual Theme & Styling
//!
//! Provides the consistent dark-mode color palette, metric constants,
//! and visual styles across all GUI Toolkit widgets.

use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};

pub const FONT_W: u32 = FONT_WIDTH as u32;
pub const FONT_H: u32 = FONT_HEIGHT as u32;

pub const BG_SURFACE: u32 = 0xFF1A1D23;
pub const BG_PANEL: u32 = 0xFF14171D;
pub const BG_INPUT: u32 = 0xFF121418;

pub const BORDER_DEFAULT: u32 = 0xFF2A2E38;
pub const BORDER_HOVER: u32 = 0xFF425670;
pub const BORDER_FOCUSED: u32 = 0xFF528BFF;
pub const BORDER_DISABLED: u32 = 0xFF20232A;

pub const ACCENT_PRIMARY: u32 = 0xFF528BFF;
pub const ACCENT_HOVER: u32 = 0xFF6B9DFF;
pub const ACCENT_PRESSED: u32 = 0xFF3D72E0;

// Button Styles
pub const BTN_PRIMARY_BG: u32 = 0xFF528BFF;
pub const BTN_PRIMARY_HOVER: u32 = 0xFF6B9DFF;
pub const BTN_PRIMARY_PRESSED: u32 = 0xFF3D72E0;

pub const BTN_SECONDARY_BG: u32 = 0xFF2B3A4C;
pub const BTN_SECONDARY_HOVER: u32 = 0xFF384B62;
pub const BTN_SECONDARY_PRESSED: u32 = 0xFF202C3A;

pub const BTN_DANGER_BG: u32 = 0xFFE05353;
pub const BTN_DANGER_HOVER: u32 = 0xFFEB6868;
pub const BTN_DANGER_PRESSED: u32 = 0xFFC93E3E;

pub const BTN_SUCCESS_BG: u32 = 0xFF4CAF50;
pub const BTN_SUCCESS_HOVER: u32 = 0xFF5DBE61;
pub const BTN_SUCCESS_PRESSED: u32 = 0xFF3D9140;

pub const BTN_DISABLED_BG: u32 = 0xFF1E222A;

// Typography Colors
pub const TEXT_PRIMARY: u32 = 0xFFFFFFFF;
pub const TEXT_SECONDARY: u32 = 0xFF8B939E;
pub const TEXT_DISABLED: u32 = 0xFF555B66;
pub const TEXT_ACCENT: u32 = 0xFF528BFF;

// Control Metrics
pub const CHECKBOX_SIZE: u32 = 16;
pub const RADIO_RADIUS: i32 = 7;
pub const SLIDER_TRACK_HEIGHT: u32 = 4;
pub const SLIDER_THUMB_RADIUS: i32 = 6;
pub const PROGRESS_BAR_HEIGHT: u32 = 14;

// Sliders and Progress Bars
pub const PROGRESS_BG: u32 = 0xFF121418;
pub const PROGRESS_FILL: u32 = 0xFF528BFF;
pub const PROGRESS_BORDER: u32 = 0xFF2A2E38;

pub const SLIDER_TRACK_BG: u32 = 0xFF2A2E38;
pub const SLIDER_TRACK_FILL: u32 = 0xFF528BFF;
pub const SLIDER_THUMB_BG: u32 = 0xFF528BFF;
pub const SLIDER_THUMB_BORDER: u32 = 0xFFFFFFFF;
pub const SLIDER_THUMB_HOVER: u32 = 0xFF6B9DFF;
pub const SLIDER_THUMB_PRESSED: u32 = 0xFF3D72E0;
