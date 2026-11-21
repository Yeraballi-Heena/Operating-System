//! Integrate OS — Desktop Environment Verification Suite (Phase 20)
//!
//! Rigorous 5-stage test suite verifying:
//! 1. Desktop icon grid layout, hover, selection, and double-click launching
//! 2. Start Menu anchoring, open/close lifecycle, outside dismissal, and item hit-testing
//! 3. Application window instantiation and WindowManager integration
//! 4. Desktop Shell multi-layer event routing hierarchy
//! 5. Full desktop scene composition and pixel readback on Canvas surface

use crate::graphics::canvas::Canvas;
use crate::desktop::icons::DesktopIconGrid;
use crate::desktop::shell::DesktopShell;
use crate::desktop::start_menu::{StartMenu, StartMenuHitResult};
use crate::desktop::types::{AppId, DesktopAction, IconState};
use crate::gui::theme::BG_PANEL;
use crate::input::events::MouseButton;
use crate::wm::theme::DESKTOP_BG_TOP;
use crate::wm::WindowManager;
use crate::{klog_info, klog_ok};

pub fn run_desktop_tests() {
    klog_info!("Starting Phase 20 Integrate OS Desktop Verification Suite...");

    // =========================================================================
    // Test 1: Desktop Icon Grid Layout, Hover, Selection & Double-Click Launch
    // =========================================================================
    let mut grid = DesktopIconGrid::new();
    if grid.icons.len() != 4 {
        panic!("Desktop Test 1 Failed: Expected 4 default desktop icons, got {}", grid.icons.len());
    }

    // Verify ordering and layout coordinates
    if grid.icons[0].app_id != AppId::Terminal || grid.icons[0].bounds.x != 24 || grid.icons[0].bounds.y != 24 {
        panic!("Desktop Test 1 Failed: Icon 0 geometry mismatch: {:?}", grid.icons[0].bounds);
    }
    if grid.icons[1].app_id != AppId::FileManager || grid.icons[1].bounds.y != 24 + 82 + 16 {
        panic!("Desktop Test 1 Failed: Icon 1 geometry mismatch: {:?}", grid.icons[1].bounds);
    }

    // Hover state update
    grid.update_hover(30, 30);
    if grid.icons[0].state != IconState::Hover || grid.icons[1].state != IconState::Normal {
        panic!("Desktop Test 1 Failed: Hover state mismatch");
    }

    // Single Click: selection at tick 100
    let action1 = grid.handle_click(30, 30, 100);
    if action1 != DesktopAction::None || grid.icons[0].state != IconState::Selected {
        panic!("Desktop Test 1 Failed: Single click should select icon");
    }

    // Double Click: second click within 50 ticks (tick 125, delta = 25)
    let action2 = grid.handle_click(30, 30, 125);
    if action2 != DesktopAction::LaunchApp(AppId::Terminal) {
        panic!("Desktop Test 1 Failed: Double click should launch Terminal, got {:?}", action2);
    }

    // Click on empty desktop background (x = 400, y = 400): deselects
    grid.handle_click(400, 400, 200);
    if grid.icons[0].state != IconState::Normal {
        panic!("Desktop Test 1 Failed: Background click should deselect icon");
    }

    klog_ok!("Phase 20 Test 1 Passed: Desktop icon grid layout, hover & double-click launch verified");

    // =========================================================================
    // Test 2: Start Menu Anchoring, Open/Close Lifecycle & Item Hit-Testing
    // =========================================================================
    let mut menu = StartMenu::new(1024, 768);
    let expected_y = 768 - 30 - 264 - 4; // 470
    if menu.bounds.x != 4 || menu.bounds.y != expected_y as i32 || menu.bounds.width != 224 || menu.bounds.height != 264 {
        panic!("Desktop Test 2 Failed: Start menu bounds mismatch: {:?}", menu.bounds);
    }

    if menu.is_open {
        panic!("Desktop Test 2 Failed: Start menu should start closed");
    }

    menu.toggle();
    if !menu.is_open {
        panic!("Desktop Test 2 Failed: Toggle failed to open start menu");
    }

    // Outside click dismissal
    let outside_action = menu.handle_click(500, 500);
    if outside_action != DesktopAction::None || menu.is_open {
        panic!("Desktop Test 2 Failed: Outside click should close start menu");
    }

    menu.open();

    // Hit test first item (Terminal)
    let item0_rect = menu.items[0].rect;
    let hit_item0 = menu.hit_test(item0_rect.x + 4, item0_rect.y + 4);
    if hit_item0 != StartMenuHitResult::AppItem(AppId::Terminal) {
        panic!("Desktop Test 2 Failed: Expected AppItem(Terminal), got {:?}", hit_item0);
    }

    // Hit test Shutdown and Reboot
    let hit_sd = menu.hit_test(menu.shutdown_rect.x + 2, menu.shutdown_rect.y + 2);
    if hit_sd != StartMenuHitResult::Shutdown {
        panic!("Desktop Test 2 Failed: Expected Shutdown hit, got {:?}", hit_sd);
    }

    let hit_rb = menu.hit_test(menu.reboot_rect.x + 2, menu.reboot_rect.y + 2);
    if hit_rb != StartMenuHitResult::Reboot {
        panic!("Desktop Test 2 Failed: Expected Reboot hit, got {:?}", hit_rb);
    }

    // Clicking Shutdown returns DesktopAction::Shutdown and closes menu
    let sd_action = menu.handle_click(menu.shutdown_rect.x + 2, menu.shutdown_rect.y + 2);
    if sd_action != DesktopAction::Shutdown || menu.is_open {
        panic!("Desktop Test 2 Failed: Shutdown click failed");
    }

    klog_ok!("Phase 20 Test 2 Passed: Start Menu anchoring, state transitions & hit-testing verified");

    // =========================================================================
    // Test 3: Application Window Spawning & WindowManager Integration
    // =========================================================================
    let mut wm = WindowManager::new(1024, 768);
    let mut shell = DesktopShell::new(1024, 768);

    // Launch File Manager
    let win_files = shell.launch_app(AppId::FileManager, &mut wm);
    if wm.windows.len() != 1 {
        panic!("Desktop Test 3 Failed: Window not added to WindowManager");
    }

    let win = wm.find_window(win_files).expect("Failed to locate launched window");
    if win.title != "File Manager" || win.bounds.width != 520 || win.bounds.height != 360 {
        panic!("Desktop Test 3 Failed: Launched window properties mismatch: {:?}", win.bounds);
    }

    // Launch Terminal
    let win_term = shell.launch_app(AppId::Terminal, &mut wm);
    if wm.windows.len() != 2 {
        panic!("Desktop Test 3 Failed: Expected 2 windows");
    }

    // Verify focus elevation: Terminal should be active and at the top of the stack
    let active_win = wm.find_window(win_term).expect("Terminal not found");
    if !active_win.active || wm.windows.last().unwrap().id != win_term {
        panic!("Desktop Test 3 Failed: Launched window did not receive top focus");
    }

    klog_ok!("Phase 20 Test 3 Passed: Application window spawning & WindowManager integration verified");

    // =========================================================================
    // Test 4: Desktop Shell Multi-Layer Event Routing
    // =========================================================================
    // 4a. Click on a region occluded by an open window should be ignored by desktop icons
    // Active Terminal window covers (140, 80) to (700, 460)
    let action_occluded = shell.handle_mouse_click(200, 200, MouseButton::Left, &mut wm, 300);
    if action_occluded != DesktopAction::None {
        panic!("Desktop Test 4 Failed: Window occluded click was consumed by desktop");
    }

    // 4b. Open Start Menu and verify outside click dismisses it
    shell.start_menu.open();
    let action_outside = shell.handle_mouse_click(500, 100, MouseButton::Left, &mut wm, 310);
    if action_outside != DesktopAction::None || shell.start_menu.is_open {
        panic!("Desktop Test 4 Failed: Start menu outside click dismissal failed");
    }

    // 4c. Toggle Start Menu
    shell.toggle_start_menu();
    if !shell.start_menu.is_open {
        panic!("Desktop Test 4 Failed: toggle_start_menu failed to open");
    }
    shell.toggle_start_menu();
    if shell.start_menu.is_open {
        panic!("Desktop Test 4 Failed: toggle_start_menu failed to close");
    }

    klog_ok!("Phase 20 Test 4 Passed: Desktop Shell multi-layer event routing verified");

    // =========================================================================
    // Test 5: Full Desktop Scene Composition & Pixel Readback on Canvas
    // =========================================================================
    let mut test_canvas = Canvas::new(1024, 768, 1024 * 4, 32, 0);

    // 5a. Render wallpaper and icons
    shell.render_background(&mut test_canvas);

    // Verify top-left desktop background pixel matches DESKTOP_BG_TOP (0xFF111D28)
    let px_top = test_canvas.get_pixel(0, 0).expect("Failed to read top background pixel");
    if px_top.to_raw() != DESKTOP_BG_TOP {
        panic!("Desktop Test 5 Failed: Top background color mismatch: expected {:#010x}, got {:#010x}",
            DESKTOP_BG_TOP, px_top.to_raw());
    }

    // Verify bottom-left desktop background pixel (above taskbar, y = 730) matches interpolated gradient 0xFF1A2B3B
    let px_bottom = test_canvas.get_pixel(0, 730).expect("Failed to read bottom background pixel");
    if px_bottom.to_raw() != 0xFF1A2B3B {
        panic!("Desktop Test 5 Failed: Bottom background gradient mismatch: expected 0xFF1A2B3B, got {:#010x}",
            px_bottom.to_raw());
    }

    // Verify Terminal desktop icon body pixel (icon center at x = 24 + 38 = 62, y = 24 + 20 = 44) matches 0xFF1E222A
    let px_icon = test_canvas.get_pixel(62, 44).expect("Failed to read icon pixel");
    if px_icon.to_raw() != 0xFF1E222A {
        panic!("Desktop Test 5 Failed: Terminal icon background mismatch: expected 0xFF1E222A, got {:#010x}",
            px_icon.to_raw());
    }

    // 5b. Open Start Menu and render overlay
    shell.start_menu.open();
    shell.render_overlay(&mut test_canvas);

    // Verify Start Menu background pixel inside card (x = 50, y = 550) matches BG_PANEL (0xFF14171D)
    let px_menu = test_canvas.get_pixel(50, 550).expect("Failed to read start menu pixel");
    if px_menu.to_raw() != BG_PANEL {
        panic!("Desktop Test 5 Failed: Start menu background mismatch: expected {:#010x}, got {:#010x}",
            BG_PANEL, px_menu.to_raw());
    }

    // Verify Start Menu header banner pixel (x = 50, y = 480) matches 0xFF1E2633
    let px_header = test_canvas.get_pixel(50, 480).expect("Failed to read start menu header pixel");
    if px_header.to_raw() != 0xFF1E2633 {
        panic!("Desktop Test 5 Failed: Start menu header mismatch: expected 0xFF1E2633, got {:#010x}",
            px_header.to_raw());
    }

    // Explicitly drop test_canvas to immediately free back buffer
    drop(test_canvas);

    klog_ok!("Phase 20 Test 5 Passed: Full desktop scene composition & pixel readback verified");
    klog_ok!("PHASE 20 VERIFIED: Integrate OS Desktop (Icons, Start Menu, Watermark, App Launch) operational");
}
