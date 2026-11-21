//! Integrate OS — Central Desktop Shell Coordinator
//!
//! Ties together wallpaper rendering, desktop icons, Start Menu popup,
//! and application launching with the Window Manager.

use crate::graphics::canvas::Canvas;
use crate::graphics::rect::Rect;
use crate::desktop::icons::DesktopIconGrid;
use crate::desktop::start_menu::StartMenu;
use crate::desktop::types::{AppId, DesktopAction};
use crate::desktop::wallpaper::render_wallpaper;
use crate::wm::types::*;
use crate::wm::WindowManager;
use crate::input::events::MouseButton;

pub struct DesktopShell {
    pub screen_width: u32,
    pub screen_height: u32,
    pub icons: DesktopIconGrid,
    pub start_menu: StartMenu,
}

impl DesktopShell {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            screen_width: width,
            screen_height: height,
            icons: DesktopIconGrid::new(),
            start_menu: StartMenu::new(width, height),
        }
    }

    /// Handles mouse motion across the desktop shell.
    pub fn handle_mouse_move(&mut self, x: i32, y: i32, wm: &WindowManager) {
        if self.start_menu.is_open {
            self.start_menu.update_hover(x, y);
        } else {
            // Only hover desktop icons if not covered by a visible window
            let mut covered = false;
            for win in wm.windows.iter().rev() {
                if win.state != WindowState::Minimized && win.state != WindowState::Hidden && win.bounds.contains(x, y) {
                    covered = true;
                    break;
                }
            }
            if !covered {
                self.icons.update_hover(x, y);
            }
        }
    }

    /// Handles mouse press/click on the desktop environment.
    pub fn handle_mouse_click(
        &mut self,
        x: i32,
        y: i32,
        button: MouseButton,
        wm: &mut WindowManager,
        tick: u64,
    ) -> DesktopAction {
        if button != MouseButton::Left {
            return DesktopAction::None;
        }

        // 1. If Start Menu is open, process it first
        if self.start_menu.is_open {
            if self.start_menu.bounds.contains(x, y) {
                let action = self.start_menu.handle_click(x, y);
                if let DesktopAction::LaunchApp(app) = action {
                    self.launch_app(app, wm);
                }
                return action;
            } else {
                // Clicked outside: close start menu
                self.start_menu.close();
                // If clicked on taskbar start button region, toggle closes it (consumed)
                let start_btn_rect = Rect::new(0, (self.screen_height - 30) as i32, 90, 30);
                if start_btn_rect.contains(x, y) {
                    return DesktopAction::ToggleStartMenu;
                }
            }
        }

        // 2. Check if a visible window captures the click
        for win in wm.windows.iter().rev() {
            if win.state != WindowState::Minimized && win.state != WindowState::Hidden && win.bounds.contains(x, y) {
                return DesktopAction::None;
            }
        }

        // 3. Click is on the desktop background: dispatch to Desktop Icons
        let action = self.icons.handle_click(x, y, tick);
        if let DesktopAction::LaunchApp(app) = action {
            self.launch_app(app, wm);
        }
        action
    }

    /// Toggles the Start Menu popup open/closed.
    pub fn toggle_start_menu(&mut self) {
        self.start_menu.toggle();
    }

    /// Launches a desktop application by instantiating its window in the WindowManager.
    pub fn launch_app(&self, app: AppId, wm: &mut WindowManager) -> WindowId {
        let (title, x, y, w, h) = match app {
            AppId::Pc => ("PC - Hardware, Mounted SSD & WiFi", 120, 48, 620, 420),
            AppId::Terminal => ("Terminal", 140, 80, 560, 380),
            AppId::FileManager => ("File Manager", 180, 110, 520, 360),
            AppId::SystemMonitor => ("System Monitor", 220, 140, 500, 340),
            AppId::TextEditor => ("Text Editor", 200, 120, 540, 360),
            AppId::PackageManager => ("APT & Snap Package Manager", 160, 90, 580, 380),
            AppId::Settings => ("Settings", 240, 160, 480, 320),
            AppId::Games => ("Games - Arcade Hub & Retro Snake", 140, 60, 560, 400),
            AppId::MediaPlayer => ("Media Player - HD Video & Sound", 160, 70, 580, 420),
        };

        let win_id = wm.create_window(title, x, y, w, h, WindowFlags::DEFAULT);

        // Pre-populate client surface with initial app view
        if let Some(win) = wm.find_window_mut(win_id) {
            match app {
                AppId::Pc | AppId::PackageManager | AppId::Terminal => {
                    let bg = 0xFF14171D;
                    win.fill_client(bg);
                    win.draw_client_string(10, 10, "integrate@os:~$ uname -a", 0xFF4CAF50, bg);
                    win.draw_client_string(10, 30, "IntegrateOS 0.1.0 x86_64 Long Mode", 0xFFE5E7EB, bg);
                    win.draw_client_string(10, 50, "integrate@os:~$ _", 0xFF4CAF50, bg);
                }
                AppId::SystemMonitor => {
                    let bg = 0xFF161920;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "SYSTEM DIAGNOSTICS & RESOURCES", 0xFF528BFF, bg);
                    win.draw_client_string(12, 34, "Architecture : x86_64 Long Mode (Ring 0)", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 54, "Kernel Heap  : 32 MiB Mapped / 64 MiB Max", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 74, "Display Mode : 1024x768x32 Linear Framebuffer", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 94, "Filesystem   : /boot, /dev, /ramfs active", 0xFFFFFFFF, bg);
                }
                AppId::FileManager => {
                    let bg = 0xFF181B22;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "FILE SYSTEM EXPLORER [/]", 0xFF528BFF, bg);
                    win.draw_client_string(12, 36, "[DIR]  boot/", 0xFF60A5FA, bg);
                    win.draw_client_string(12, 56, "[DIR]  dev/", 0xFF60A5FA, bg);
                    win.draw_client_string(12, 76, "[DIR]  ramfs/", 0xFF60A5FA, bg);
                    win.draw_client_string(12, 96, "[FILE] integrate-os.bin", 0xFFE5E7EB, bg);
                }
                AppId::Settings => {
                    let bg = 0xFF1A1D24;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "INTEGRATE OS CONTROL PANEL", 0xFF528BFF, bg);
                    win.draw_client_string(12, 36, "Display  : Bochs / VBox VBE (1024x768)", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 56, "Theme    : Integrate Dark / Slate Blue", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 76, "Keyboard : PS/2 Set 1 US-QWERTY", 0xFFFFFFFF, bg);
                    win.draw_client_string(12, 96, "Mouse    : PS/2 Streaming 3-Byte IRQ12", 0xFFFFFFFF, bg);
                }
                AppId::TextEditor => {
                    let bg = 0xFF121418;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "// Integrate OS Scratchpad", 0xFF8B939E, bg);
                    win.draw_client_string(12, 32, "fn main() {", 0xFF528BFF, bg);
                    win.draw_client_string(12, 52, "    println!(\"Hello from Integrate OS!\");", 0xFFE5E7EB, bg);
                    win.draw_client_string(12, 72, "}", 0xFF528BFF, bg);
                }
                AppId::Games => {
                    let bg = 0xFF13111C;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "ARCADE GAMES SUITE", 0xFFC4B5FD, bg);
                }
                AppId::MediaPlayer => {
                    let bg = 0xFF0B0F19;
                    win.fill_client(bg);
                    win.draw_client_string(12, 12, "INTEGRATE OS MEDIA PLAYER", 0xFF38BDF8, bg);
                }
            }
        }

        let _ = wm.focus_window(win_id);
        win_id
    }

    /// Renders Layer 0 background: wallpaper gradient, watermark, and desktop icons.
    pub fn render_background(&self, canvas: &mut Canvas) {
        render_wallpaper(canvas);
        self.icons.render(canvas);
    }

    /// Renders Layer 4 overlay: Start Menu popup if open.
    pub fn render_overlay(&self, canvas: &mut Canvas) {
        if self.start_menu.is_open {
            self.start_menu.render(canvas);
        }
    }
}
