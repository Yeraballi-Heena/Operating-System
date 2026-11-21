//! Integrate OS — Desktop Environment Types & Action Enums
//!
//! Provides application identifiers, desktop action commands,
//! and icon visual interaction states.

/// Standard built-in applications in Integrate OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppId {
    Pc,
    Terminal,
    FileManager,
    SystemMonitor,
    TextEditor,
    PackageManager,
    Settings,
    Games,
    MediaPlayer,
}

impl AppId {
    pub const fn title(self) -> &'static str {
        match self {
            AppId::Pc => "PC",
            AppId::Terminal => "Terminal",
            AppId::FileManager => "File Manager",
            AppId::SystemMonitor => "System Monitor",
            AppId::TextEditor => "Text Editor",
            AppId::PackageManager => "Packages",
            AppId::Settings => "Settings",
            AppId::Games => "Games",
            AppId::MediaPlayer => "Media Player",
        }
    }
}

/// Actions dispatched by desktop shell user interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopAction {
    None,
    LaunchApp(AppId),
    Shutdown,
    Reboot,
    ToggleStartMenu,
}

/// Visual state of a desktop icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Normal,
    Hover,
    Selected,
}
