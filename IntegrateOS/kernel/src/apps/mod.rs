//! Integrate OS — Desktop Applications Subsystem
//!
//! Provides the primary user-facing graphical and terminal desktop applications:
//! - PC (`PcApp` — Mounted SSD, Hardware IDs, IP & WiFi Center)
//! - Terminal (`TerminalApp`)
//! - File Manager (`FileManagerApp`)
//! - System Monitor (`SystemMonitorApp`)
//! - Text Editor (`TextEditorApp`)
//! - Snap Store / Package Manager (`SnapStoreApp`)
//! - Settings Control Panel (`SettingsApp`)

pub mod editor;
pub mod file_manager;
pub mod games;
pub mod media_player;
pub mod pc;
pub mod settings;
pub mod snapstore;
pub mod snapstore_tests;
pub mod sysmon;
pub mod terminal;
pub mod tests;

pub use editor::TextEditorApp;
pub use file_manager::FileManagerApp;
pub use games::GamesApp;
pub use media_player::MediaPlayerApp;
pub use pc::PcApp;
pub use settings::SettingsApp;
pub use snapstore::SnapStoreApp;
pub use sysmon::SystemMonitorApp;
pub use terminal::TerminalApp;

use crate::desktop::types::AppId;
use crate::framework::manager::AppManager;
use crate::framework::types::{AppError, InstanceId};
use crate::wm::WindowManager;
use crate::klog_ok;

/// Initializes the desktop applications subsystem.
pub fn init() {
    klog_ok!("[apps] Desktop Application Suite initialized (PC, Terminal, Files, Monitor, Packages, Games, Media, Settings)");
}

/// Spawns an application instance by its desktop AppId into the runtime AppManager and WindowManager.
pub fn spawn_by_id(
    app: AppId,
    app_mgr: &mut AppManager,
    wm: &mut WindowManager,
) -> Result<InstanceId, AppError> {
    match app {
        AppId::Pc => app_mgr.spawn(PcApp::new(), wm, 106, 30, 646, 366),
        AppId::Terminal => app_mgr.spawn(TerminalApp::new(), wm, 370, 398, 642, 332),
        AppId::FileManager => app_mgr.spawn(FileManagerApp::new(), wm, 180, 110, 520, 360),
        AppId::SystemMonitor => app_mgr.spawn(SystemMonitorApp::new(), wm, 220, 140, 500, 340),
        AppId::TextEditor => app_mgr.spawn(TextEditorApp::new(), wm, 200, 120, 540, 360),
        AppId::PackageManager => app_mgr.spawn(SnapStoreApp::new(), wm, 160, 80, 620, 380),
        AppId::Settings => app_mgr.spawn(SettingsApp::new(), wm, 240, 160, 480, 320),
        AppId::Games => app_mgr.spawn(GamesApp::new(), wm, 140, 60, 560, 400),
        AppId::MediaPlayer => app_mgr.spawn(MediaPlayerApp::new(), wm, 160, 70, 580, 420),
    }
}
