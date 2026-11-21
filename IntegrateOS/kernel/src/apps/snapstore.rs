//! Integrate OS — In-Built App Store (`snapstore` / `snap`)
//!
//! Provides CLI and GUI package discovery, sandboxed snap distribution,
//! installation, launching, and lifecycle management.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

#[derive(Debug, Clone)]
pub struct SnapPackage {
    pub name: String,
    pub version: String,
    pub developer: &'static str,
    pub channel: &'static str,
    pub summary: String,
    pub installed_size_mb: u32,
    pub category: &'static str,
    pub rating_stars: u8,
    pub installed: bool,
}

impl SnapPackage {
    pub fn new(
        name: &str,
        version: &str,
        developer: &'static str,
        category: &'static str,
        size_mb: u32,
        rating: u8,
        summary: &str,
        installed: bool,
    ) -> Self {
        Self {
            name: String::from(name),
            version: String::from(version),
            developer,
            channel: "stable/latest",
            summary: String::from(summary),
            installed_size_mb: size_mb,
            category,
            rating_stars: rating,
            installed,
        }
    }
}

pub struct SnapStoreManager {
    pub catalog: BTreeMap<String, SnapPackage>,
}

impl SnapStoreManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            catalog: BTreeMap::new(),
        };

        mgr.register(SnapPackage::new("firefox", "124.0.2", "Mozilla", "Productivity", 128, 5, "Fast, Private & Safe Web Browser", true));
        mgr.register(SnapPackage::new("code", "1.87.2", "Microsoft", "Development", 145, 5, "Visual Studio Code - Editing evolved", false));
        mgr.register(SnapPackage::new("vlc", "3.0.20", "VideoLAN", "Multimedia", 78, 4, "Ultimate media player and streaming engine", true));
        mgr.register(SnapPackage::new("spotify", "1.2.31", "Spotify", "Multimedia", 92, 5, "Music for everyone - millions of tracks", false));
        mgr.register(SnapPackage::new("discord", "0.0.48", "Discord", "Social", 88, 4, "All-in-one voice and text chat for teams", false));
        mgr.register(SnapPackage::new("gimp", "2.10.36", "GIMP Team", "Graphics", 112, 4, "GNU Image Manipulation Program", false));
        mgr.register(SnapPackage::new("wireshark", "4.2.4", "Wireshark Foundation", "Utilities", 56, 5, "Network traffic capture and deep packet analysis", false));
        mgr.register(SnapPackage::new("htop-snap", "3.3.0", "Hisham Muhammad", "System", 3, 5, "Interactive process monitor with CPU affinity view", true));

        mgr
    }

    pub fn register(&mut self, snap: SnapPackage) {
        self.catalog.insert(snap.name.clone(), snap);
    }

    pub fn find(&self, name: &str) -> Option<&SnapPackage> {
        self.catalog.get(name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut SnapPackage> {
        self.catalog.get_mut(name)
    }

    pub fn total_count(&self) -> usize {
        self.catalog.len()
    }

    pub fn installed_count(&self) -> usize {
        self.catalog.values().filter(|s| s.installed).count()
    }

    pub fn snap_install(&mut self, name: &str) -> (bool, String) {
        if let Some(snap) = self.catalog.get_mut(name) {
            if snap.installed {
                return (true, format!("snap \"{}\" is already installed (channel \"{}\")\n", name, snap.channel));
            }
            snap.installed = true;
            klog_info!("[SNAP] Installed snap package '{}' ({})", name, snap.version);
            (true, format!("{name} {ver} from {dev} installed ({mb} MB)\n",
                name = snap.name, ver = snap.version, dev = snap.developer, mb = snap.installed_size_mb))
        } else {
            (false, format!("error: snap \"{}\" not found\n", name))
        }
    }

    pub fn snap_remove(&mut self, name: &str) -> (bool, String) {
        if let Some(snap) = self.catalog.get_mut(name) {
            if !snap.installed {
                return (false, format!("error: snap \"{}\" is not installed\n", name));
            }
            snap.installed = false;
            klog_info!("[SNAP] Removed snap package '{}'", name);
            (true, format!("{name} removed\n", name = snap.name))
        } else {
            (false, format!("error: snap \"{}\" not found\n", name))
        }
    }

    pub fn snap_list(&self) -> String {
        let mut out = String::new();
        out.push_str("Name         Version    Rev    Tracking       Publisher    Notes\n");
        for (name, snap) in &self.catalog {
            if snap.installed {
                out.push_str(&format!("{:<12} {:<10} 1042   {:<14} {:<12} classic\n",
                    name, snap.version, snap.channel, snap.developer));
            }
        }
        out
    }

    pub fn snap_find(&self, query: &str) -> String {
        let mut out = String::new();
        let query_lower = query.to_ascii_lowercase();
        out.push_str("Name         Version    Publisher    Notes    Summary\n");
        let mut count = 0;
        for (name, snap) in &self.catalog {
            if query.is_empty()
                || name.to_ascii_lowercase().contains(&query_lower)
                || snap.summary.to_ascii_lowercase().contains(&query_lower)
                || snap.category.to_ascii_lowercase().contains(&query_lower)
            {
                out.push_str(&format!("{:<12} {:<10} {:<12} -        {}\n",
                    name, snap.version, snap.developer, snap.summary));
                count += 1;
            }
        }
        if count == 0 {
            out.push_str(&format!("No snaps matching \"{}\".\n", query));
        }
        out
    }

    pub fn snap_info(&self, name: &str) -> String {
        if let Some(snap) = self.catalog.get(name) {
            let mut out = String::new();
            out.push_str(&format!("name:    {}\n", snap.name));
            out.push_str(&format!("summary: {}\n", snap.summary));
            out.push_str(&format!("publisher: {}\n", snap.developer));
            out.push_str(&format!("tracking:  {}\n", snap.channel));
            out.push_str(&format!("installed: {} ({} MB) {}\n",
                if snap.installed { "yes" } else { "no" },
                snap.installed_size_mb,
                if snap.installed { "[active]" } else { "" }
            ));
            out.push_str(&format!("channels:\n  latest/stable: {} (rev 1042)\n", snap.version));
            out
        } else {
            format!("error: no snap found for \"{}\"\n", name)
        }
    }

    pub fn dispatch_command(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::from("The snap command lets you install, configure, refresh and remove snaps.\nUsage: snap <command> [<options>...]\nCommands: find, info, install, remove, list\n");
        }

        match args[0] {
            "list" => self.snap_list(),
            "find" | "search" => {
                let q = if args.len() > 1 { args[1] } else { "" };
                self.snap_find(q)
            }
            "info" => {
                if args.len() > 1 {
                    self.snap_info(args[1])
                } else {
                    String::from("error: please provide a snap name\n")
                }
            }
            "install" => {
                if args.len() > 1 {
                    self.snap_install(args[1]).1
                } else {
                    String::from("error: please provide a snap name to install\n")
                }
            }
            "remove" => {
                if args.len() > 1 {
                    self.snap_remove(args[1]).1
                } else {
                    String::from("error: please provide a snap name to remove\n")
                }
            }
            "help" | "--help" | "-h" => {
                String::from("Snap Store CLI v2.61.3\nCommands:\n  find <query>    Find packages in Snap Store\n  info <name>     Show detailed snap metadata\n  install <name>  Install a snap package\n  remove <name>   Remove a snap package\n  list            List installed snaps\n")
            }
            unknown => format!("error: unknown command \"{}\"\n", unknown),
        }
    }
}

pub static SNAP_MANAGER: IrqSpinlock<Option<SnapStoreManager>> = IrqSpinlock::new(None);

/// Initializes the Snap Store subsystem.
pub fn init() {
    let mut guard = SNAP_MANAGER.lock();
    if guard.is_none() {
        *guard = Some(SnapStoreManager::new());
    }
    let mgr = guard.as_ref().unwrap();
    klog_ok!(
        "[snapstore] Snap Store subsystem ready ({} snaps cataloged, {} installed)",
        mgr.total_count(),
        mgr.installed_count()
    );
}

/// Desktop GUI Snap Store Window Application.
pub struct SnapStoreApp {
    pub selected_index: usize,
    pub hovered_row: Option<usize>,
    pub hovered_btn: Option<usize>,
    pub pressed_btn: Option<usize>,
    pub status_msg: String,
}

impl SnapStoreApp {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            hovered_row: None,
            hovered_btn: None,
            pressed_btn: None,
            status_msg: String::from("Snap Store: Click [Get] or [Installed] to manage packages."),
        }
    }
}

impl Application for SnapStoreApp {
    fn name(&self) -> &str {
        "Snap Store"
    }

    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError> {
        env.set_title("Snap Store - Software Package Center");
        env.request_redraw();
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        use crate::gui::types::WidgetEvent;
        use crate::input::events::MouseButton;
        use crate::audio::speaker::PcSpeaker;

        match *event {
            AppEvent::Input(WidgetEvent::MouseMove { x, y }) => {
                let prev_r = self.hovered_row;
                let prev_b = self.hovered_btn;

                if y >= 76 {
                    let row = ((y - 76) / 42) as usize;
                    self.hovered_row = Some(row);

                    // Check if hovering Get/Installed button on right
                    if x >= 480 {
                        self.hovered_btn = Some(row);
                    } else {
                        self.hovered_btn = None;
                    }
                } else {
                    self.hovered_row = None;
                    self.hovered_btn = None;
                }

                if prev_r != self.hovered_row || prev_b != self.hovered_btn {
                    env.request_redraw();
                }
            }
            AppEvent::Input(WidgetEvent::MouseDown { y, button: MouseButton::Left, .. }) => {
                self.pressed_btn = self.hovered_btn;

                if y >= 76 {
                    let row = ((y - 76) / 42) as usize;
                    self.selected_index = row;

                    // Get/Installed button clicked
                    let mut guard = SNAP_MANAGER.lock();
                    if let Some(mgr) = guard.as_mut() {
                        let names: Vec<String> = mgr.catalog.keys().cloned().collect();
                        if let Some(name) = names.get(row) {
                            let is_installed = mgr.catalog.get(name).map(|s| s.installed).unwrap_or(false);
                            if is_installed {
                                let (_ok, msg) = mgr.snap_remove(name);
                                self.status_msg = msg;
                                PcSpeaker::beep(440, 40);
                            } else {
                                let (_ok, msg) = mgr.snap_install(name);
                                self.status_msg = msg;
                                PcSpeaker::beep(880, 50);
                            }
                        }
                    }
                    env.request_redraw();
                }
            }
            AppEvent::Input(WidgetEvent::MouseUp { .. }) => {
                self.pressed_btn = None;
                env.request_redraw();
            }
            _ => {}
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}

    fn render(&self, canvas: &mut Canvas) {
        let w = canvas.width;
        let h = canvas.height;

        // App Store Background: Slate Dark (#1e1e2e)
        canvas.draw_rect_fill(Rect::new(0, 0, w, h), Color32::rgb(30, 30, 46));

        // Header Banner: Integrate Accent (#E95420)
        canvas.draw_rect_fill(Rect::new(0, 0, w, 44), Color32::rgb(233, 84, 32));
        canvas.draw_text(16, 14, "Snap Store - Software Center", Color32::WHITE, None);
        canvas.draw_text((w.saturating_sub(260)) as i32, 14, "Explore | Installed | Updates", Color32::rgb(240, 240, 240), None);

        // Categories bar
        canvas.draw_rect_fill(Rect::new(0, 44, w, 26), Color32::rgb(40, 42, 54));
        canvas.draw_text(16, 50, "Categories: [All]  Productivity  Development  Multimedia  Utilities", Color32::rgb(180, 180, 200), None);

        // Render Snap Cards
        let guard = SNAP_MANAGER.lock();
        if let Some(mgr) = guard.as_ref() {
            let mut y: i32 = 76;
            for (i, (_name, snap)) in mgr.catalog.iter().enumerate() {
                if (y + 40) as u32 > h {
                    break;
                }

                let is_hover = self.hovered_row == Some(i);
                let card_bg = if i == self.selected_index {
                    Color32::rgb(55, 59, 82)
                } else if is_hover {
                    Color32::rgb(45, 48, 68)
                } else {
                    Color32::rgb(36, 38, 54)
                };

                let card_rect = Rect::new(16, y, w.saturating_sub(32), 36);
                canvas.draw_rect_fill(card_rect, card_bg);
                if i == self.selected_index {
                    canvas.draw_rect_outline(card_rect, Color32::rgb(0x3B, 0x82, 0xF6));
                }

                // App Name & Version (bounded)
                let title = format!("{} v{}", snap.name, snap.version);
                canvas.draw_text(28, y + 10, &title, Color32::WHITE, None);

                // Category pill
                canvas.draw_rect_fill(Rect::new(180, y + 8, 70, 20), Color32::rgb(24, 25, 38));
                canvas.draw_text(184, y + 10, snap.category, Color32::rgb(140, 150, 180), None);

                // Summary (truncated so it never collides with button)
                let max_sum_len = (w.saturating_sub(380) / 8) as usize;
                let mut sum_text = snap.summary.clone();
                if sum_text.len() > max_sum_len {
                    sum_text.truncate(max_sum_len.saturating_sub(3));
                    sum_text.push_str("...");
                }
                canvas.draw_text(260, y + 10, &sum_text, Color32::rgb(190, 195, 210), None);

                // Action Button with hover & press animation
                let is_btn_hover = self.hovered_btn == Some(i);
                let is_btn_press = self.pressed_btn == Some(i);
                let off = if is_btn_press { 1 } else { 0 };

                let (status_text, base_bg, border_col) = if snap.installed {
                    ("Installed", Color32::rgb(16, 185, 129), Color32::rgb(110, 231, 183))
                } else {
                    ("Get", Color32::rgb(37, 99, 235), Color32::rgb(147, 197, 253))
                };

                let btn_bg = if is_btn_hover {
                    Color32::rgb(base_bg.r.saturating_add(25), base_bg.g.saturating_add(25), base_bg.b.saturating_add(25))
                } else {
                    base_bg
                };

                let bx = (w.saturating_sub(105)) as i32 + off;
                let by = y + 6 + off;
                canvas.draw_rect_fill(Rect::new(bx, by, 84, 24), btn_bg);
                canvas.draw_rect_outline(Rect::new(bx, by, 84, 24), border_col);
                canvas.draw_text(bx + 14, by + 4, status_text, Color32::WHITE, None);

                y += 42;
            }
        }

        // Footer / Status Bar
        canvas.draw_rect_fill(Rect::new(0, (h.saturating_sub(28)) as i32, w, 28), Color32::rgb(24, 24, 37));
        canvas.draw_text(16, (h.saturating_sub(20)) as i32, &self.status_msg, Color32::rgb(200, 200, 200), None);
    }
}
