//! Phase 18 — Compositor Verification Suite
//!
//! Validates:
//! 1. Damage tracking, bounding box computation, and dirty rectangle merging
//! 2. Alpha blending calculations and window drop shadow rendering
//! 3. Layered scene composition hierarchy (Background -> Shadows -> Windows -> Taskbar -> Cursor)
//! 4. Taskbar hit-testing, active window synchronization, and state toggling
//! 5. Full-frame composition performance and damage-driven VRAM presentation metrics

use crate::compositor::damage::DamageRegion;
use crate::compositor::effects::{apply_opacity, blend_argb, draw_drop_shadow};
use crate::compositor::engine::CompositorEngine;
use crate::compositor::taskbar::TASKBAR_HEIGHT;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::wm::types::*;
use crate::wm::WindowManager;
use crate::{klog_info, klog_ok};

/// Executes the comprehensive Phase 18 Compositor verification suite.
pub fn run_compositor_tests() {
    klog_info!("Starting Phase 18 Compositor Verification Suite...");

    // Test 1: Damage Tracking & Dirty Rectangle Merging
    let mut damage = DamageRegion::new(1024, 768);
    assert!(!damage.is_dirty());
    assert!(damage.bounding_box().is_none());

    // 1a. Add first dirty rect
    damage.add_rect(Rect::new(50, 50, 100, 100));
    assert!(damage.is_dirty());
    let bbox1 = damage.bounding_box().unwrap();
    if bbox1 != Rect::new(50, 50, 100, 100) {
        panic!("Compositor Test 1 Failed: Initial bounding box mismatch: {:?}", bbox1);
    }

    // 1b. Add non-overlapping disjoint rect
    damage.add_rect(Rect::new(300, 300, 100, 100));
    let bbox2 = damage.bounding_box().unwrap();
    // Bounding union of (50,50,100,100) and (300,300,100,100) is (50,50,350,350)
    if bbox2.x != 50 || bbox2.y != 50 || bbox2.width != 350 || bbox2.height != 350 {
        panic!("Compositor Test 1 Failed: Disjoint bounding box mismatch: {:?}", bbox2);
    }

    // 1c. Add overlapping rect to verify merging
    damage.add_rect(Rect::new(80, 80, 100, 100));
    // Should merge with the (50, 50, 100, 100) rect
    if damage.rects().len() > 2 {
        panic!("Compositor Test 1 Failed: Overlapping rectangle was not merged");
    }

    // 1d. Add full screen and clear
    damage.add_fullscreen();
    if damage.bounding_box() != Some(Rect::new(0, 0, 1024, 768)) {
        panic!("Compositor Test 1 Failed: Fullscreen damage mismatch");
    }
    damage.clear();
    assert!(!damage.is_dirty());
    assert!(damage.bounding_box().is_none());

    klog_ok!("Phase 18 Test 1 Passed: Damage tracking & dirty rectangle merging verified");

    // Test 2: Alpha Blending Calculations & Drop Shadow Generation
    // 2a. Opaque source
    let c1 = blend_argb(0xFF_FF0000, 0xFF_00FF00);
    if c1 != 0xFF_FF0000 {
        panic!("Compositor Test 2 Failed: Opaque source blend mismatch: {:#010x}", c1);
    }

    // 2b. Fully transparent source
    let c2 = blend_argb(0x00_FF0000, 0xFF_00FF00);
    if c2 != 0xFF_00FF00 {
        panic!("Compositor Test 2 Failed: Transparent source blend mismatch: {:#010x}", c2);
    }

    // 2c. 50% alpha blend (white over black)
    let c3 = blend_argb(0x80_FFFFFF, 0xFF_000000);
    let r = (c3 >> 16) & 0xFF;
    let g = (c3 >> 8) & 0xFF;
    let b = c3 & 0xFF;
    if (r as i32 - 128).abs() > 2 || (g as i32 - 128).abs() > 2 || (b as i32 - 128).abs() > 2 {
        panic!("Compositor Test 2 Failed: 50% alpha blend channel mismatch: {:#010x}", c3);
    }

    // 2d. Opacity scaling
    let c_op = apply_opacity(0xFF_112233, 128);
    let alpha_scaled = (c_op >> 24) & 0xFF;
    if (alpha_scaled as i32 - 128).abs() > 2 {
        panic!("Compositor Test 2 Failed: Opacity scaling mismatch: alpha={}", alpha_scaled);
    }

    // 2e. Render drop shadow on a test canvas and check shadow perimeter
    let mut test_canvas = Canvas::new(500, 500, 2000, 32, 0);
    test_canvas.clear(Color32::WHITE);
    draw_drop_shadow(&mut test_canvas, Rect::new(100, 100, 200, 200), 8, true);

    // Pixel at (100 + 200 + 2, 100 + 100) is within the right shadow area; should be darkened
    let shadow_pixel = test_canvas.get_pixel(303, 150).expect("Failed to get shadow pixel");
    if shadow_pixel.r == 255 && shadow_pixel.g == 255 && shadow_pixel.b == 255 {
        panic!("Compositor Test 2 Failed: Shadow did not darken canvas pixel");
    }

    klog_ok!("Phase 18 Test 2 Passed: Alpha blending math & window drop shadow generation verified");

    // Test 3: Layered Scene Composition Hierarchy
    let mut wm = WindowManager::new(1024, 768);
    // Create a window that overlaps the bottom taskbar region
    let win_id = wm.create_window(
        "Overlap Test",
        100,
        (768 - TASKBAR_HEIGHT - 50) as i32,
        300,
        100,
        WindowFlags::DEFAULT,
    );

    let mut comp = CompositorEngine::new(1024, 768);
    let mut scene_canvas = Canvas::new(1024, 768, 4096, 32, 0);

    // Compose the full multi-layer scene
    comp.compose(&mut scene_canvas, &wm, (512, 384), 250);

    // Check pixel at (150, 755) which is inside taskbar bounds where the window would overlap.
    // The taskbar layer MUST occlude the window layer.
    let taskbar_pixel = scene_canvas.get_pixel(150, 755).expect("Failed to read taskbar pixel");
    if taskbar_pixel.to_raw() == 0xFF1A1D23 {
        // 0xFF1A1D23 is default window client color
        panic!("Compositor Test 3 Failed: Window occluded Taskbar; Layer order inverted!");
    }
    drop(scene_canvas);

    klog_ok!("Phase 18 Test 3 Passed: Multi-layer scene composition hierarchy verified");

    // Test 4: Taskbar Hit-Testing, Window List Synchronization & State Toggling
    let tb = &comp.taskbar;

    // 4a. Start button hit-test
    let sb_rect = tb.start_button_rect();
    let res_start = tb.hit_test(sb_rect.x + 2, sb_rect.y + 2, &wm);
    if res_start != crate::compositor::taskbar::TaskbarHitResult::StartButton {
        panic!("Compositor Test 4 Failed: Expected StartButton hit, got {:?}", res_start);
    }

    // 4b. Task item hit-test for window 0
    let item_rect = tb.task_item_rect(0);
    let res_item = tb.hit_test(item_rect.x + 2, item_rect.y + 2, &wm);
    if res_item != crate::compositor::taskbar::TaskbarHitResult::TaskItem(win_id) {
        panic!("Compositor Test 4 Failed: Expected TaskItem hit, got {:?}", res_item);
    }

    // 4c. Click taskbar item: since window is currently active, clicking should minimize it
    let consumed = comp.handle_mouse_click(&mut wm, item_rect.x + 2, item_rect.y + 2);
    assert!(consumed);
    if wm.find_window(win_id).unwrap().state != WindowState::Minimized {
        panic!("Compositor Test 4 Failed: Clicking taskbar item did not minimize active window");
    }

    // 4d. Click taskbar item again: window is minimized, so clicking should restore it
    let consumed2 = comp.handle_mouse_click(&mut wm, item_rect.x + 2, item_rect.y + 2);
    assert!(consumed2);
    if wm.find_window(win_id).unwrap().state != WindowState::Normal {
        panic!("Compositor Test 4 Failed: Clicking taskbar item did not restore minimized window");
    }

    klog_ok!("Phase 18 Test 4 Passed: Taskbar hit-testing & window state toggling verified");

    // Test 5: Full Frame Composition Performance & Presentation Metrics
    let mut perf_canvas = Canvas::new(1024, 768, 4096, 32, 0);
    let initial_frames = comp.stats.total_frames;

    // Mark dirty region and compose
    comp.mark_dirty(Rect::new(200, 200, 300, 200));
    assert!(comp.is_dirty());

    comp.compose(&mut perf_canvas, &wm, (100, 100), 500);

    if comp.stats.total_frames != initial_frames + 1 {
        panic!("Compositor Test 5 Failed: Total frames metric did not increment");
    }
    if comp.stats.dirty_pixels_flushed == 0 {
        panic!("Compositor Test 5 Failed: Zero dirty pixels recorded in presentation stats");
    }
    assert!(!comp.is_dirty()); // Damage cleared after present

    klog_ok!("Phase 18 Test 5 Passed: Compositor performance stats & partial VRAM presentation verified");
    klog_ok!("PHASE 18 VERIFIED: Compositor (Damage Tracking, Alpha Blending, Taskbar, Layers) operational");
}
