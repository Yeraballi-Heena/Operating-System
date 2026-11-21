//! Integrate OS — Desktop Environment Subsystem (Phase 20)
//!
//! Provides the complete desktop shell environment:
//! - Desktop branding wallpaper & geometric emblem watermark (`wallpaper.rs`)
//! - Interactive desktop shortcut icons with vector glyphs and double-click launching (`icons.rs`)
//! - Anchored Start Menu application launcher & power actions (`start_menu.rs`)
//! - Central DesktopShell coordinator managing window spawning and event routing (`shell.rs`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod icons;
pub mod runtime;
pub mod shell;
pub mod start_menu;
pub mod types;
pub mod wallpaper;
pub mod tests;

pub use icons::{DesktopIcon, DesktopIconGrid};
pub use shell::DesktopShell;
pub use start_menu::{StartMenu, StartMenuHitResult};
pub use types::{AppId, DesktopAction, IconState};
pub use wallpaper::render_wallpaper;

use crate::klog_ok;
use crate::sync::IrqSpinlock;

pub static DESKTOP_SHELL: IrqSpinlock<Option<DesktopShell>> = IrqSpinlock::new(None);

/// Initializes the Desktop Environment subsystem (1024x768 default).
pub fn init() {
    let shell = DesktopShell::new(1024, 768);
    *DESKTOP_SHELL.lock() = Some(shell);
    klog_ok!("Desktop Shell Initialized: Wallpaper watermark, desktop icons & Start Menu active");
}
