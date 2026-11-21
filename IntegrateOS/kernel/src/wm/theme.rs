//! Window Manager Visual Theme & Decoration Styling
//!
//! Provides color palettes and dimension metrics for window title bars, borders,
//! control buttons, and the desktop backdrop.

pub const TITLE_BAR_HEIGHT: u32 = 26;
pub const BORDER_WIDTH: u32 = 2;
pub const RESIZE_HANDLE_SIZE: u32 = 6;
pub const BUTTON_SIZE: u32 = 14;
pub const BUTTON_MARGIN_RIGHT: u32 = 6;
pub const BUTTON_SPACING: u32 = 6;

// Desktop Colors
pub const DESKTOP_BG_TOP: u32 = 0xFF111D28;
pub const DESKTOP_BG_BOTTOM: u32 = 0xFF1B2C3D;

// Window Border Colors
pub const BORDER_COLOR_ACTIVE: u32 = 0xFF425670;
pub const BORDER_COLOR_INACTIVE: u32 = 0xFF272B33;

// Title Bar Colors
pub const TITLE_BAR_ACTIVE_TOP: u32 = 0xFF2C3B4D;
pub const TITLE_BAR_ACTIVE_BOTTOM: u32 = 0xFF1C2531;
pub const TITLE_BAR_INACTIVE: u32 = 0xFF202329;
pub const TITLE_TEXT_ACTIVE: u32 = 0xFFFFFFFF;
pub const TITLE_TEXT_INACTIVE: u32 = 0xFF8B939E;

// Control Button Colors
pub const BTN_CLOSE: u32 = 0xFFE05353;
pub const BTN_MAXIMIZE: u32 = 0xFF4CAF50;
pub const BTN_MINIMIZE: u32 = 0xFFFFA000;

// Client Area Colors
pub const CLIENT_BG_DEFAULT: u32 = 0xFF1A1D23;
