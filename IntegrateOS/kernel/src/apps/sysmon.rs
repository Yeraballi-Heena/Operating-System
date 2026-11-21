//! Integrate OS — Real-Time System Monitor Application
//!
//! Provides live telemetry and visual diagnostic meters for physical RAM frames,
//! kernel heap utilization, CPU timer ticks, process counts, and system architecture.

use alloc::string::String;
use crate::arch::x86_64::pit;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};
use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::heap::ALLOCATOR;
use crate::process::manager::PROCESS_MANAGER;

/// Real-time System Monitor Application.
pub struct SystemMonitorApp {
    pub ram_total_kb: usize,
    pub ram_used_kb: usize,
    pub ram_pct: u32,
    pub heap_total_kb: usize,
    pub heap_used_kb: usize,
    pub heap_pct: u32,
    pub uptime_secs: u64,
    pub uptime_ticks: u64,
    pub process_count: usize,
    pub elapsed_update_ms: u32,
    pub active_tab: usize, // 0 = Resources, 1 = System Info
}

impl SystemMonitorApp {
    pub fn new() -> Self {
        let mut app = Self {
            ram_total_kb: 0,
            ram_used_kb: 0,
            ram_pct: 0,
            heap_total_kb: 0,
            heap_used_kb: 0,
            heap_pct: 0,
            uptime_secs: 0,
            uptime_ticks: 0,
            process_count: 1,
            elapsed_update_ms: 0,
            active_tab: 0,
        };
        app.refresh_metrics();
        app
    }

    /// Queries hardware allocators and timer subsystems to update live metrics.
    pub fn refresh_metrics(&mut self) {
        // 1. Physical RAM Metrics
        let frame_alloc = FRAME_ALLOCATOR.lock();
        let total_frames = frame_alloc.total_frames();
        let free_frames = frame_alloc.free_frames();
        drop(frame_alloc);

        let used_frames = total_frames.saturating_sub(free_frames);
        self.ram_total_kb = (total_frames * PAGE_SIZE) / 1024;
        self.ram_used_kb = (used_frames * PAGE_SIZE) / 1024;
        self.ram_pct = if total_frames > 0 {
            ((used_frames as u64 * 100) / total_frames as u64) as u32
        } else {
            0
        };

        // 2. Kernel Heap Metrics
        let heap_stats = ALLOCATOR.0.lock();
        let alloc_bytes = heap_stats.allocated_bytes();
        let free_bytes = heap_stats.free_bytes();
        drop(heap_stats);

        let total_heap = alloc_bytes + free_bytes;
        self.heap_total_kb = total_heap / 1024;
        self.heap_used_kb = alloc_bytes / 1024;
        self.heap_pct = if total_heap > 0 {
            ((alloc_bytes as u64 * 100) / total_heap as u64) as u32
        } else {
            0
        };

        // 3. CPU and Timer
        let ticks = pit::get_ticks();
        self.uptime_ticks = ticks;
        self.uptime_secs = ticks / 100;

        // 4. Process Count
        let proc_mgr = PROCESS_MANAGER.lock();
        self.process_count = proc_mgr.count();
        drop(proc_mgr);
    }
}

impl Application for SystemMonitorApp {
    fn name(&self) -> &str {
        "System Monitor"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        self.refresh_metrics();
        Ok(())
    }

    fn on_update(&mut self, env: &mut AppEnv, delta_ms: u32) {
        self.elapsed_update_ms += delta_ms;
        // Poll every 500 ms
        if self.elapsed_update_ms >= 500 {
            self.elapsed_update_ms = 0;
            self.refresh_metrics();
            env.request_redraw();
        }
    }

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Input(ref we) = *event {
            match *we {
                WidgetEvent::KeyDown { code, .. } => {
                    if code == KeyCode::F5 || code == KeyCode::Enter {
                        self.refresh_metrics();
                        env.request_redraw();
                    } else if code == KeyCode::Tab {
                        self.active_tab = (self.active_tab + 1) % 2;
                        env.request_redraw();
                    }
                }
                WidgetEvent::MouseDown { x, y, button } => {
                    if button == MouseButton::Left && y >= 6 && y <= 32 {
                        // Tab 0: Resources (10..110)
                        if x >= 10 && x <= 110 {
                            self.active_tab = 0;
                            env.request_redraw();
                        }
                        // Tab 1: System Info (118..220)
                        else if x >= 118 && x <= 220 {
                            self.active_tab = 1;
                            env.request_redraw();
                        }
                        // Refresh button (canvas.width - 80 .. canvas.width - 10)
                        else if x >= 230 && x <= 310 {
                            self.refresh_metrics();
                            env.request_redraw();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}

    fn render(&self, canvas: &mut Canvas) {
        let bg_color = Color32::rgb(0x16, 0x19, 0x20);
        canvas.clear(bg_color);

        let h = canvas.height as i32;

        // 1. Header Toolbar & Tabs
        let header_bg = Color32::rgb(0x20, 0x25, 0x30);
        canvas.draw_rect_fill(Rect::new(0, 0, canvas.width, 38), header_bg);
        canvas.draw_hline(0, 38, canvas.width, Color32::rgb(0x2E, 0x34, 0x40));

        // Tab: Resources
        let tab0_bg = if self.active_tab == 0 {
            Color32::rgb(0x38, 0x8B, 0xF6)
        } else {
            Color32::rgb(0x2A, 0x30, 0x3C)
        };
        canvas.draw_rect_fill(Rect::new(10, 6, 100, 26), tab0_bg);
        canvas.draw_text(22, 11, "Resources", Color32::rgb(0xFF, 0xFF, 0xFF), None);

        // Tab: System Info
        let tab1_bg = if self.active_tab == 1 {
            Color32::rgb(0x38, 0x8B, 0xF6)
        } else {
            Color32::rgb(0x2A, 0x30, 0x3C)
        };
        canvas.draw_rect_fill(Rect::new(118, 6, 100, 26), tab1_bg);
        canvas.draw_text(126, 11, "System Info", Color32::rgb(0xFF, 0xFF, 0xFF), None);

        // Refresh Button
        canvas.draw_rect_fill(Rect::new(228, 6, 80, 26), Color32::rgb(0x2A, 0x30, 0x3C));
        canvas.draw_rect_outline(Rect::new(228, 6, 80, 26), Color32::rgb(0x48, 0x4F, 0x58));
        canvas.draw_text(240, 11, "Refresh", Color32::rgb(0xEB, 0xF0, 0xF4), None);

        // 2. Tab Contents
        if self.active_tab == 0 {
            // --- RESOURCES VIEW ---
            let card_w = (canvas.width.saturating_sub(32)) as u32;

            // Section 1: Physical RAM
            let y1 = 48;
            canvas.draw_text(16, y1, "Physical RAM Utilization", Color32::rgb(0x58, 0xA6, 0xFF), None);

            // Progress bar
            let bar_y1 = y1 + 22;
            let bar_h = 20;
            canvas.draw_rect_fill(Rect::new(16, bar_y1, card_w, bar_h), Color32::rgb(0x0E, 0x11, 0x17));
            canvas.draw_rect_outline(Rect::new(16, bar_y1, card_w, bar_h), Color32::rgb(0x30, 0x36, 0x3D));

            let fill_w1 = ((card_w as u64 * self.ram_pct as u64) / 100).min(card_w as u64) as u32;
            if fill_w1 > 0 {
                let gauge_color = if self.ram_pct > 85 {
                    Color32::rgb(0xFA, 0x52, 0x52)
                } else if self.ram_pct > 65 {
                    Color32::rgb(0xFA, 0xAD, 0x14)
                } else {
                    Color32::rgb(0x38, 0x8B, 0xF6)
                };
                canvas.draw_rect_fill(Rect::new(16, bar_y1, fill_w1, bar_h), gauge_color);
            }

            // RAM Stats text
            let mut ram_info = String::from("Used: ");
            let used_mib = self.ram_used_kb / 1024;
            let total_mib = self.ram_total_kb / 1024;
            // append numbers
            format_num(&mut ram_info, used_mib as u64);
            ram_info.push_str(" MiB / ");
            format_num(&mut ram_info, total_mib as u64);
            ram_info.push_str(" MiB (");
            format_num(&mut ram_info, self.ram_pct as u64);
            ram_info.push_str("%)");
            canvas.draw_text(16, bar_y1 + 26, &ram_info, Color32::rgb(0x8B, 0x94, 0x9E), None);

            // Section 2: Kernel Heap Memory
            let y2 = bar_y1 + 54;
            canvas.draw_text(16, y2, "Kernel Heap Utilization", Color32::rgb(0xBC, 0x8C, 0xFF), None);

            let bar_y2 = y2 + 22;
            canvas.draw_rect_fill(Rect::new(16, bar_y2, card_w, bar_h), Color32::rgb(0x0E, 0x11, 0x17));
            canvas.draw_rect_outline(Rect::new(16, bar_y2, card_w, bar_h), Color32::rgb(0x30, 0x36, 0x3D));

            let fill_w2 = ((card_w as u64 * self.heap_pct as u64) / 100).min(card_w as u64) as u32;
            if fill_w2 > 0 {
                canvas.draw_rect_fill(Rect::new(16, bar_y2, fill_w2, bar_h), Color32::rgb(0xBC, 0x8C, 0xFF));
            }

            let mut heap_info = String::from("Allocated: ");
            format_num(&mut heap_info, self.heap_used_kb as u64);
            heap_info.push_str(" KiB / Total: ");
            format_num(&mut heap_info, self.heap_total_kb as u64);
            heap_info.push_str(" KiB (");
            format_num(&mut heap_info, self.heap_pct as u64);
            heap_info.push_str("%)");
            canvas.draw_text(16, bar_y2 + 26, &heap_info, Color32::rgb(0x8B, 0x94, 0x9E), None);

            // Section 3: CPU & PIT Timer
            let y3 = bar_y2 + 54;
            canvas.draw_text(16, y3, "CPU PIT Heartbeat & Activity", Color32::rgb(0x3F, 0xB9, 0x50), None);

            let bar_y3 = y3 + 22;
            canvas.draw_rect_fill(Rect::new(16, bar_y3, card_w, bar_h), Color32::rgb(0x0E, 0x11, 0x17));
            canvas.draw_rect_outline(Rect::new(16, bar_y3, card_w, bar_h), Color32::rgb(0x30, 0x36, 0x3D));

            // Animated / dynamic ticker pulse
            let pulse = ((self.uptime_ticks % 100) as u32 * card_w) / 100;
            canvas.draw_rect_fill(Rect::new(16, bar_y3, pulse, bar_h), Color32::rgb(0x3F, 0xB9, 0x50));

            let mut cpu_info = String::from("Heartbeat: ");
            format_num(&mut cpu_info, self.uptime_ticks);
            cpu_info.push_str(" ticks | Frequency: 100 Hz | Uptime: ");
            format_num(&mut cpu_info, self.uptime_secs);
            cpu_info.push_str(" s");
            canvas.draw_text(16, bar_y3 + 26, &cpu_info, Color32::rgb(0x8B, 0x94, 0x9E), None);

        } else {
            // --- SYSTEM INFO VIEW ---
            let mut y = 54;
            let step = 26;

            draw_info_row(canvas, 16, y, "Operating System", "Integrate OS 0.1.0 (Pure Rust)");
            y += step;
            draw_info_row(canvas, 16, y, "Target Architecture", "x86_64 Long Mode (Ring 0)");
            y += step;
            draw_info_row(canvas, 16, y, "Memory Paging", "4-Level Hierarchical Paging (CR3)");
            y += step;
            draw_info_row(canvas, 16, y, "Graphics Engine", "Double-Buffered Canvas (1024x768)");
            y += step;
            draw_info_row(canvas, 16, y, "VFS Filesystem", "Hierarchical RAMFS / dev / boot");
            y += step;

            let mut proc_str = String::from("Processes Active: ");
            format_num(&mut proc_str, self.process_count as u64);
            draw_info_row(canvas, 16, y, "Process Subsystem", &proc_str);
            y += step;

            let mut uptime_str = String::from("");
            format_num(&mut uptime_str, self.uptime_secs);
            uptime_str.push_str(" seconds (");
            format_num(&mut uptime_str, self.uptime_ticks);
            uptime_str.push_str(" PIT ticks)");
            draw_info_row(canvas, 16, y, "System Uptime", &uptime_str);
        }

        // Bottom status bar
        let status_y = h - 22;
        canvas.draw_rect_fill(Rect::new(0, status_y, canvas.width, 22), Color32::rgb(0x10, 0x12, 0x16));
        canvas.draw_hline(0, status_y, canvas.width, Color32::rgb(0x28, 0x2E, 0x38));
        canvas.draw_text(12, status_y + 3, "Telemetry status: Active (polling @ 2 Hz)", Color32::rgb(0x58, 0xA6, 0xFF), None);
    }
}

fn draw_info_row(canvas: &mut Canvas, x: i32, y: i32, label: &str, val: &str) {
    canvas.draw_text(x, y, label, Color32::rgb(0x58, 0xA6, 0xFF), None);
    canvas.draw_text(x + 200, y, val, Color32::rgb(0xEB, 0xF0, 0xF4), None);
}

fn format_num(s: &mut String, val: u64) {
    if val == 0 {
        s.push('0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut len = 0;
    let mut temp = val;
    while temp > 0 {
        digits[len] = b'0' + (temp % 10) as u8;
        temp /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        s.push(digits[i] as char);
    }
}
