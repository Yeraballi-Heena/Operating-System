//! Integrate OS — Live Interactive Desktop Runtime Loop (Phases 8–16)
//!
//! Connects hardware interrupts & 8042 PS/2 input polling -> InputEventQueue
//! -> DesktopShell (Icons, Start Menu) & Taskbar -> WindowManager -> AppManager
//! -> Multi-Layer Compositor -> BGA Linear Framebuffer VRAM (`0xE000_0000`).

use crate::apps::spawn_by_id;
use crate::arch::x86_64::{keyboard, pit};
use crate::compositor::effects::draw_drop_shadow;
use crate::compositor::engine::CompositorEngine;
use crate::compositor::taskbar::TaskbarHitResult;
use crate::desktop::shell::DesktopShell;
use crate::desktop::types::{AppId, DesktopAction};
use crate::framework::event::AppEvent;
use crate::framework::manager::AppManager;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::cursor::VirtualCursor;
use crate::input::events::{
    InputEventKind, KeyAction, KeyCode, MouseButton, MouseButtonAction,
};
use crate::klog_ok;
use crate::wm::render::render_window;
use crate::wm::types::{HitTestResult, WindowFlags, WindowState};
use crate::wm::WindowManager;

/// Spawns an application if not already open, or focuses & restores its window if already running.
fn spawn_or_focus_app(app: AppId, app_mgr: &mut AppManager, wm: &mut WindowManager) {
    let target_title_prefix = match app {
        AppId::Pc => "PC",
        AppId::Terminal => "Terminal",
        AppId::FileManager => "File Manager",
        AppId::SystemMonitor => "System Monitor",
        AppId::TextEditor => "Text Editor",
        AppId::PackageManager => "Snap Store",
        AppId::Settings => "Settings",
        AppId::Games => "Games",
        AppId::MediaPlayer => "Media Player",
    };

    let existing_win_id = wm
        .windows
        .iter()
        .find(|w| w.title.starts_with(target_title_prefix))
        .map(|w| w.id);

    if let Some(win_id) = existing_win_id {
        let _ = wm.restore_window(win_id);
        let _ = wm.focus_window(win_id);
    } else {
        let _ = spawn_by_id(app, app_mgr, wm);
    }
}

/// Starts the live interactive graphical desktop environment and enters the main event loop.
pub fn run_interactive_desktop() -> ! {
    // 1. Ensure high-resolution 1024x768x32 BGA Linear Framebuffer mode is active
    let _ = crate::graphics::init(1024, 768, 32);

    let mut shell = DesktopShell::new(1024, 768);
    shell.icons.init_live_desktop();

    let mut wm = WindowManager::new(1024, 768);
    let mut app_mgr = AppManager::new();
    let mut comp = CompositorEngine::new(1024, 768);

    // Startup apps disabled by user request: Clean boot directly to desktop & icons!
    // Apps are launched on-demand via desktop icons, Start Menu, or shortcut keys.

    // Clear any stale synthetic test input events from boot self-tests
    crate::input::clear_events();
    let _ = keyboard::drain_8042_controller();

    // Center initial mouse cursor, configure screen boundaries, and enable desktop sensitivity
    crate::input::mouse::MOUSE.lock().set_screen_bounds(1024, 768);
    crate::input::mouse::MOUSE.lock().cursor.set_position(512, 384);
    crate::input::mouse::MOUSE.lock().set_sensitivity(true);

    klog_ok!("[desktop-runtime] Clean Graphical Desktop Active (1024x768 @ 32bpp, 8 Icons, Mouse & Apps Ready)");

    let mut last_tick = pit::get_ticks();
    let mut last_clock_redraw_tick = last_tick;
    let mut needs_redraw = true;

    loop {
        // 1. Drain 8042 PS/2 controller for any pending keyboard or mouse bytes
        let _ = keyboard::drain_8042_controller();

        // 2. Process all queued input events
        while let Some(ev) = crate::input::pop_event() {
            needs_redraw = true;

            match ev.kind {
                InputEventKind::MouseMove { position: (mx, my), .. } => {
                    // Window moving / resizing
                    wm.handle_mouse_move((mx, my));
                    if let crate::wm::manager::InteractionState::Resizing { window_id, .. } =
                        wm.interaction
                    {
                        if let Some(inst_id) = app_mgr.find_by_window(window_id) {
                            if let Some(inst) = app_mgr.find_mut(inst_id) {
                                AppManager::render_instance_ui(inst, &mut wm);
                            }
                        }
                    }

                    // Desktop icon & start menu hover
                    shell.handle_mouse_move(mx, my, &wm);

                    // Forward MouseMove to active window's application
                    if let Some(active_id) = wm.active_window {
                        if let Some(win) = wm.find_window(active_id) {
                            let cr = win.client_rect();
                            if cr.contains(mx, my) {
                                let rx = mx - cr.x;
                                let ry = my - cr.y;
                                if let Some(inst_id) = app_mgr.find_by_window(active_id) {
                                    let _ = app_mgr.dispatch_event(
                                        inst_id,
                                        &AppEvent::Input(WidgetEvent::MouseMove { x: rx, y: ry }),
                                        &mut wm,
                                    );
                                }
                            }
                        }
                    }
                }
                InputEventKind::MouseButton {
                    button,
                    action: MouseButtonAction::Press,
                    position: (mx, my),
                } => {
                    if button == MouseButton::Left {
                        // A. Check Taskbar first (bottom 30px)
                        if comp.taskbar.bounds().contains(mx, my) {
                            match comp.taskbar.hit_test(mx, my, &wm) {
                                TaskbarHitResult::StartButton => {
                                    shell.toggle_start_menu();
                                    comp.taskbar.start_active = shell.start_menu.is_open;
                                }
                                TaskbarHitResult::TaskItem(win_id) => {
                                    shell.start_menu.close();
                                    comp.taskbar.start_active = false;
                                    if let Some(win) = wm.find_window(win_id) {
                                        if win.active && win.state != WindowState::Minimized {
                                            let _ = wm.minimize_window(win_id);
                                        } else if win.state == WindowState::Minimized {
                                            let _ = wm.restore_window(win_id);
                                        } else {
                                            let _ = wm.focus_window(win_id);
                                        }
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // B. Check Start Menu popup if open
                        if shell.start_menu.is_open {
                            if shell.start_menu.bounds.contains(mx, my) {
                                let act = shell.start_menu.handle_click(mx, my);
                                comp.taskbar.start_active = shell.start_menu.is_open;
                                match act {
                                    DesktopAction::LaunchApp(app_id) => {
                                        spawn_or_focus_app(app_id, &mut app_mgr, &mut wm);
                                    }
                                    DesktopAction::Shutdown => {
                                        crate::acpi::power_off();
                                    }
                                    DesktopAction::Reboot => {
                                        crate::acpi::reboot();
                                    }
                                    _ => {}
                                }
                                continue;
                            } else {
                                shell.start_menu.close();
                                comp.taskbar.start_active = false;
                            }
                        }

                        // C. Check Windows (top-to-bottom Z-order)
                        if let Some((win_id, ht)) = wm.hit_test(mx, my) {
                            let _ = wm.focus_window(win_id);
                            match ht {
                                HitTestResult::CloseButton => {
                                    if let Some(inst_id) = app_mgr.find_by_window(win_id) {
                                        let _ = app_mgr.terminate(inst_id, &mut wm);
                                    } else {
                                        let _ = wm.destroy_window(win_id);
                                    }
                                }
                                HitTestResult::ClientArea => {
                                    if let Some(win) = wm.find_window(win_id) {
                                        let cr = win.client_rect();
                                        let rx = mx - cr.x;
                                        let ry = my - cr.y;
                                        if let Some(inst_id) = app_mgr.find_by_window(win_id) {
                                            let _ = app_mgr.dispatch_event(
                                                inst_id,
                                                &AppEvent::Input(WidgetEvent::MouseDown {
                                                    x: rx,
                                                    y: ry,
                                                    button,
                                                }),
                                                &mut wm,
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    wm.handle_mouse_button(
                                        button,
                                        MouseButtonAction::Press,
                                        (mx, my),
                                    );
                                }
                            }
                            continue;
                        }

                        // D. Click on Desktop Background / Desktop Icons
                        let act = shell.icons.handle_live_click(mx, my, ev.timestamp);
                        if let DesktopAction::LaunchApp(app_id) = act {
                            spawn_or_focus_app(app_id, &mut app_mgr, &mut wm);
                        }
                    }
                }
                InputEventKind::MouseButton {
                    button,
                    action: MouseButtonAction::Release,
                    position: (mx, my),
                } => {
                    wm.handle_mouse_button(button, MouseButtonAction::Release, (mx, my));
                    if let Some(active_id) = wm.active_window {
                        if let Some(win) = wm.find_window(active_id) {
                            let cr = win.client_rect();
                            let rx = mx - cr.x;
                            let ry = my - cr.y;
                            if let Some(inst_id) = app_mgr.find_by_window(active_id) {
                                let _ = app_mgr.dispatch_event(
                                    inst_id,
                                    &AppEvent::Input(WidgetEvent::MouseUp {
                                        x: rx,
                                        y: ry,
                                        button,
                                    }),
                                    &mut wm,
                                );
                            }
                        }
                    }
                }
                InputEventKind::Key(key_ev) => {
                    if key_ev.action == KeyAction::Press || key_ev.action == KeyAction::Repeat {
                        // Function-key quick application launchers
                        match key_ev.code {
                            KeyCode::F1 => {
                                spawn_or_focus_app(AppId::Pc, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F2 => {
                                spawn_or_focus_app(AppId::Terminal, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F3 => {
                                spawn_or_focus_app(AppId::FileManager, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F4 => {
                                spawn_or_focus_app(AppId::SystemMonitor, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F5 => {
                                spawn_or_focus_app(AppId::PackageManager, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F6 => {
                                spawn_or_focus_app(AppId::Settings, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F7 => {
                                spawn_or_focus_app(AppId::Games, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::F8 => {
                                spawn_or_focus_app(AppId::MediaPlayer, &mut app_mgr, &mut wm);
                                continue;
                            }
                            KeyCode::LeftSuper | KeyCode::RightSuper => {
                                shell.toggle_start_menu();
                                comp.taskbar.start_active = shell.start_menu.is_open;
                                continue;
                            }
                            _ => {}
                        }

                        // Ensure an active window receives keyboard input
                        if wm.active_window.is_none() {
                            if let Some(top) = wm.windows.last() {
                                let top_id = top.id;
                                let _ = wm.focus_window(top_id);
                            } else {
                                spawn_or_focus_app(AppId::Terminal, &mut app_mgr, &mut wm);
                            }
                        }

                        if let Some(active_id) = wm.active_window {
                            if let Some(inst_id) = app_mgr.find_by_window(active_id) {
                                let ch = key_ev.ascii.map(|b| b as char);
                                let _ = app_mgr.dispatch_event(
                                    inst_id,
                                    &AppEvent::Input(WidgetEvent::KeyDown {
                                        code: key_ev.code,
                                        modifiers: key_ev.modifiers,
                                        character: ch,
                                    }),
                                    &mut wm,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // 3. Periodic timer update for live apps (System Monitor, Clock, Media Player, Games)
        let now_tick = pit::get_ticks();
        if now_tick > last_tick {
            let delta_ms = ((now_tick - last_tick) * 10) as u32;
            last_tick = now_tick;
            if app_mgr.update_all(delta_ms, &mut wm) {
                needs_redraw = true;
            }
        }

        if now_tick.saturating_sub(last_clock_redraw_tick) >= 100 {
            last_clock_redraw_tick = now_tick;
            needs_redraw = true;
        }

        // 4. Composite all layers and present to physical VRAM
        if needs_redraw {
            let cursor_pos = crate::input::cursor_position();

            crate::graphics::with_canvas(|canvas| {
                // Layer 1: Desktop Wallpaper & 6 Desktop Icons
                shell.render_background(canvas);

                // Top Desktop Telemetry Banner (Shifted right of desktop icons)
                canvas.draw_rect_fill(
                    Rect::new(220, 4, canvas.width.saturating_sub(228), 22),
                    Color32::rgba(15, 23, 42, 210),
                );
                canvas.draw_rect_outline(
                    Rect::new(220, 4, canvas.width.saturating_sub(228), 22),
                    Color32::rgb(0x33, 0x41, 0x55),
                );
                canvas.draw_text(
                    228,
                    7,
                    "DESKTOP-L8RF961 | Intel i7-1355U | SSD: /mnt/ssd [RO] | LAN: 10.0.2.15 | WiFi: 192.168.1.105 | [F1..F8]",
                    Color32::rgb(0xE2, 0xE8, 0xF0),
                    None,
                );

                // Layer 2 & 3: Window Drop Shadows & Z-Ordered Windows
                for win in &wm.windows {
                    if win.state == WindowState::Minimized || win.state == WindowState::Hidden {
                        continue;
                    }
                    if !win.flags.contains(WindowFlags::BORDERLESS) {
                        draw_drop_shadow(canvas, win.bounds, 6, win.active);
                    }
                    render_window(canvas, win);
                }

                // Layer 4: Desktop Taskbar
                comp.taskbar.render(canvas, &wm, now_tick);

                // Layer 5: Start Menu Popup Overlay
                shell.render_overlay(canvas);

                // Layer 6: Software Mouse Cursor
                let mut cursor = VirtualCursor::new(canvas.width as i32, canvas.height as i32);
                cursor.set_position(cursor_pos.0, cursor_pos.1);
                cursor.render(|px, py, col| {
                    canvas.draw_pixel(px, py, Color32::from_u32(col));
                });

                // Blit double-buffered back buffer to physical BGA Linear Framebuffer
                canvas.present();
            });

            needs_redraw = false;
        }

        // Wait until next interrupt (timer tick, keyboard press, or mouse movement)
        // SAFETY: Interrupts are enabled (STI); HLT wakes on IRQ 0, IRQ 1, or IRQ 12.
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
