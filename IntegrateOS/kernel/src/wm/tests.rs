//! Phase 17 — Window Manager Verification Suite
//!
//! Validates:
//! 1. Window creation, monotonic ID allocation, bounds, and client area geometry
//! 2. Z-order stacking hierarchy and dynamic focus management
//! 3. Regional hit-testing across overlapping windows and title bar control buttons
//! 4. Window translation (move/drag) and dimension resizing with minimum constraints
//! 5. Lifecycle state transitions (Maximize, Restore, Minimize, Close) and full canvas composition

use crate::graphics::canvas::Canvas;
use crate::wm::theme::*;
use crate::wm::types::*;
use crate::wm::window::Window;
use crate::wm::WindowManager;
use crate::{klog_info, klog_ok};

/// Executes the comprehensive Phase 17 Window Manager verification suite.
pub fn run_wm_tests() {
    klog_info!("Starting Phase 17 Window Manager Verification Suite...");

    // Test 1: Window Creation, Geometry, and Client Buffer Operations
    let id1 = WindowId(1);
    let mut win1 = Window::new(id1, "Terminal Console", 100, 100, 400, 300, WindowFlags::DEFAULT);

    // Verify outer geometry
    if win1.bounds.x != 100 || win1.bounds.y != 100 || win1.bounds.width != 400 || win1.bounds.height != 300 {
        panic!("WM Test 1 Failed: Outer window bounds mismatch");
    }

    // Verify client rectangle geometry
    let cr = win1.client_rect();
    let expected_cw = 400 - BORDER_WIDTH * 2;
    let expected_ch = 300 - (TITLE_BAR_HEIGHT + BORDER_WIDTH);
    let expected_cx = 100 + BORDER_WIDTH as i32;
    let expected_cy = 100 + TITLE_BAR_HEIGHT as i32;

    if cr.x != expected_cx || cr.y != expected_cy || cr.width != expected_cw || cr.height != expected_ch {
        panic!(
            "WM Test 1 Failed: Client rect mismatch: expected ({},{},{},{}), got ({},{},{},{})",
            expected_cx, expected_cy, expected_cw, expected_ch, cr.x, cr.y, cr.width, cr.height
        );
    }

    // Verify client buffer allocation
    let expected_buf_len = (expected_cw as usize) * (expected_ch as usize);
    if win1.content_buffer.len() != expected_buf_len {
        panic!(
            "WM Test 1 Failed: Content buffer size mismatch: expected {}, got {}",
            expected_buf_len,
            win1.content_buffer.len()
        );
    }

    // Test client buffer drawing
    win1.fill_client(0xFF223344);
    win1.set_client_pixel(10, 10, 0xFFFFFFFF);
    if win1.get_client_pixel(10, 10) != Some(0xFFFFFFFF) {
        panic!("WM Test 1 Failed: Client pixel readback mismatch");
    }
    win1.draw_client_string(20, 20, "Integrate OS Window", 0xFF00FF00, 0xFF223344);

    klog_ok!("Phase 17 Test 1 Passed: Window bounds, client geometry & buffer rendering verified");

    // Test 2: Z-Ordering Stacking & Dynamic Focus Transitions
    let mut wm = WindowManager::new(1024, 768);

    let w_alpha = wm.create_window("Window Alpha", 50, 50, 300, 200, WindowFlags::DEFAULT);
    let w_beta = wm.create_window("Window Beta", 100, 100, 300, 200, WindowFlags::DEFAULT);
    let w_gamma = wm.create_window("Window Gamma", 150, 150, 300, 200, WindowFlags::DEFAULT);

    if wm.windows.len() != 3 {
        panic!("WM Test 2 Failed: Expected 3 windows registered");
    }
    // Latest created window should be top of z-order and active
    if wm.active_window != Some(w_gamma) || !wm.windows[2].active {
        panic!("WM Test 2 Failed: Gamma should be active top window");
    }

    // Focus bottom window (Alpha)
    wm.focus_window(w_alpha).expect("Focus Alpha failed");
    if wm.windows[2].id != w_alpha || !wm.windows[2].active {
        panic!("WM Test 2 Failed: Alpha not elevated to top of z-order");
    }
    if wm.windows[0].active || wm.windows[1].active {
        panic!("WM Test 2 Failed: Background windows must be inactive");
    }

    // Focus middle window (Beta)
    wm.focus_window(w_beta).expect("Focus Beta failed");
    if wm.windows[2].id != w_beta || !wm.windows[2].active {
        panic!("WM Test 2 Failed: Beta not elevated to top of z-order");
    }
    // Verify full z-order: [Gamma, Alpha, Beta]
    if wm.windows[0].id != w_gamma || wm.windows[1].id != w_alpha || wm.windows[2].id != w_beta {
        panic!("WM Test 2 Failed: Z-order stack mismatch");
    }

    klog_ok!("Phase 17 Test 2 Passed: Z-order hierarchy & dynamic focus elevation verified");

    // Test 3: Hit-Testing Across Overlapping Windows
    // Currently Beta is top (100..400, 100..300), Alpha is middle (50..350, 50..250), Gamma is bottom (150..450, 150..350)
    // 3a. Click at (110, 110) - overlaps Beta title bar
    let hit_beta_tb = wm.hit_test(110, 110).expect("Hit test at (110, 110) returned None");
    if hit_beta_tb.0 != w_beta || hit_beta_tb.1 != HitTestResult::TitleBar {
        panic!("WM Test 3 Failed: Expected Beta title bar hit, got {:?}", hit_beta_tb);
    }

    // 3b. Hit test close button on Beta
    let beta_close_rect = wm.find_window(w_beta).unwrap().close_button_rect();
    let hit_close = wm.hit_test(beta_close_rect.x + 2, beta_close_rect.y + 2).expect("Hit test close button failed");
    if hit_close.0 != w_beta || hit_close.1 != HitTestResult::CloseButton {
        panic!("WM Test 3 Failed: Expected CloseButton hit, got {:?}", hit_close);
    }

    // 3c. Click at (60, 60) - Beta doesn't cover this, Alpha title bar does
    let hit_alpha = wm.hit_test(60, 60).expect("Hit test at (60, 60) returned None");
    if hit_alpha.0 != w_alpha || hit_alpha.1 != HitTestResult::TitleBar {
        panic!("WM Test 3 Failed: Expected Alpha title bar hit, got {:?}", hit_alpha);
    }

    // 3d. Click outside all windows
    let hit_outside = wm.hit_test(800, 600);
    if hit_outside.is_some() {
        panic!("WM Test 3 Failed: Expected outside click to return None");
    }

    klog_ok!("Phase 17 Test 3 Passed: Multi-layer overlapping hit-testing & control targets verified");

    // Test 4: Movement, Dragging, Resizing & Min-Dimension Clamping
    // 4a. Move window Alpha
    wm.move_window(w_alpha, 250, 320).expect("Move Alpha failed");
    let alpha_pos = wm.find_window(w_alpha).unwrap().bounds;
    if alpha_pos.x != 250 || alpha_pos.y != 320 {
        panic!("WM Test 4 Failed: Window movement position mismatch: {:?}", alpha_pos);
    }

    // 4b. Resize window Beta
    wm.resize_window(w_beta, 500, 400).expect("Resize Beta failed");
    let beta_bounds = wm.find_window(w_beta).unwrap().bounds;
    if beta_bounds.width != 500 || beta_bounds.height != 400 {
        panic!("WM Test 4 Failed: Window resize bounds mismatch: {:?}", beta_bounds);
    }
    // Verify client buffer reallocated to new size
    let (bw, bh) = wm.find_window(w_beta).unwrap().client_size();
    if wm.find_window(w_beta).unwrap().content_buffer.len() != (bw as usize) * (bh as usize) {
        panic!("WM Test 4 Failed: Client buffer size not synchronized after resize");
    }

    // 4c. Enforce minimum dimensions clamping
    wm.resize_window(w_beta, 50, 20).expect("Clamp resize failed");
    let clamped = wm.find_window(w_beta).unwrap().bounds;
    if clamped.width < 120 || clamped.height < 50 {
        panic!("WM Test 4 Failed: Minimum dimension constraints violated: {:?}", clamped);
    }

    klog_ok!("Phase 17 Test 4 Passed: Translation, resizing & minimum constraint enforcement verified");

    // Test 5: Lifecycle Transitions (Maximize, Restore, Minimize, Close) & Full Canvas Render
    // 5a. Maximize Beta
    wm.maximize_window(w_beta).expect("Maximize Beta failed");
    let max_bounds = wm.find_window(w_beta).unwrap().bounds;
    if max_bounds.width != 1024 || max_bounds.height != 768 {
        panic!("WM Test 5 Failed: Maximize bounds mismatch: {:?}", max_bounds);
    }

    // 5b. Restore Beta
    wm.restore_window(w_beta).expect("Restore Beta failed");
    let rest_bounds = wm.find_window(w_beta).unwrap().bounds;
    if rest_bounds.width != clamped.width || rest_bounds.height != clamped.height {
        panic!("WM Test 5 Failed: Restore bounds mismatch: {:?}", rest_bounds);
    }

    // 5c. Minimize Beta
    wm.minimize_window(w_beta).expect("Minimize Beta failed");
    if wm.find_window(w_beta).unwrap().state != WindowState::Minimized {
        panic!("WM Test 5 Failed: Window state is not Minimized");
    }
    // Minimized window must not be active
    if wm.find_window(w_beta).unwrap().active {
        panic!("WM Test 5 Failed: Minimized window must be inactive");
    }

    // 5d. Close/destroy Beta
    wm.destroy_window(w_beta).expect("Destroy Beta failed");
    if wm.windows.len() != 2 || wm.find_window(w_beta).is_some() {
        panic!("WM Test 5 Failed: Window Beta was not destroyed");
    }

    // 5e. Test full scene compositing onto an in-memory test Canvas
    let mut test_canvas = Canvas::new(1024, 768, 1024 * 4, 32, 0);
    wm.render(&mut test_canvas);

    // Verify desktop background pixel (top-left) was painted
    let bg_pixel = test_canvas.get_pixel(0, 0).expect("Failed to read test canvas pixel");
    if bg_pixel.to_raw() != DESKTOP_BG_TOP {
        panic!("WM Test 5 Failed: Desktop background top color mismatch: {:#010x}", bg_pixel.to_raw());
    }
    drop(test_canvas);

    klog_ok!("Phase 17 Test 5 Passed: State transitions (Max/Restore/Min/Close) & composition verified");
    klog_ok!("PHASE 17 VERIFIED: Window Manager (Z-Order, Decorations, Drag, Hit-Test) operational");
}
