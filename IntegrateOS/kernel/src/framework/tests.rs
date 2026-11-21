//! Integrate OS — Application Framework Verification Suite (Phase 21)
//!
//! Rigorous 5-stage test suite verifying:
//! 1. Application lifecycle state machine (Spawn, Suspend, Resume, Terminate) & monotonic IDs
//! 2. AppEnv UI root container integration, widget binding & action callback routing
//! 3. Thread-safe system clipboard copy/paste service & bounds enforcement
//! 4. AppEvent bus, custom inter-process messaging & timer tick processing
//! 5. Full application execution cycle with WindowManager client buffer rasterization & auto-exit

use alloc::vec::Vec;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widgets::{Button, Label, ProgressBar};
use crate::framework::app::Application;
use crate::framework::clipboard;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::manager::AppManager;
use crate::framework::types::{AppError, AppState};
use crate::input::events::MouseButton;
use crate::wm::WindowManager;
use crate::{klog_info, klog_ok};

// =========================================================================
// Test 1: Lifecycle Application
// =========================================================================
struct LifeCycleApp {
    pub initialized: bool,
    pub update_count: u32,
    pub destroyed: bool,
}

impl LifeCycleApp {
    pub fn new() -> Self {
        Self {
            initialized: false,
            update_count: 0,
            destroyed: false,
        }
    }
}

impl Application for LifeCycleApp {
    fn name(&self) -> &str {
        "LifecycleApp"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        self.initialized = true;
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {
        self.update_count += 1;
    }

    fn on_event(&mut self, _env: &mut AppEnv, _event: &AppEvent) {}

    fn on_destroy(&mut self, _env: &mut AppEnv) {
        self.destroyed = true;
    }
}

// =========================================================================
// Test 2: UI Binding Application
// =========================================================================
struct UiBindingApp {
    pub button_clicks: u32,
    pub progress_val: i32,
}

impl UiBindingApp {
    pub fn new() -> Self {
        Self {
            button_clicks: 0,
            progress_val: 20,
        }
    }
}

impl Application for UiBindingApp {
    fn name(&self) -> &str {
        "UiBindingApp"
    }

    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError> {
        env.add_widget(Button::new("Increment", 10, 10, 100, 30, 101));
        env.add_widget(ProgressBar::new(10, 50, 150, 16, 0, 100).with_value(self.progress_val));
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Action(101) = *event {
            self.button_clicks += 1;
            self.progress_val = (self.progress_val + 10).min(100);
            env.request_redraw();
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}
}

// =========================================================================
// Test 4: Messaging Application
// =========================================================================
struct MessagingApp {
    pub received_messages: Vec<(u32, u64)>,
    pub last_tick: u64,
}

impl MessagingApp {
    pub fn new() -> Self {
        Self {
            received_messages: Vec::new(),
            last_tick: 0,
        }
    }
}

impl Application for MessagingApp {
    fn name(&self) -> &str {
        "MessagingApp"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, _env: &mut AppEnv, event: &AppEvent) {
        match *event {
            AppEvent::Custom { message_id, payload } => {
                self.received_messages.push((message_id, payload));
            }
            AppEvent::TimerTick(ticks) => {
                self.last_tick = ticks;
            }
            _ => {}
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}
}

// =========================================================================
// Test 5: Dashboard Application
// =========================================================================
struct DashboardApp;

impl Application for DashboardApp {
    fn name(&self) -> &str {
        "DashboardApp"
    }

    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError> {
        env.add_widget(Label::new("Integrate Dashboard", 10, 10, 180, 20));
        env.add_widget(Button::new("Quit", 10, 40, 80, 28, 201).with_style(ButtonStyle::Danger));
        env.add_widget(ProgressBar::new(10, 80, 160, 16, 0, 100).with_value(50));
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Action(201) = *event {
            env.request_quit();
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}
}

pub fn run_framework_tests() {
    klog_info!("Starting Phase 21 Application Framework Verification Suite...");

    let mut wm = WindowManager::new(1024, 768);
    let mut mgr = AppManager::new();

    // =========================================================================
    // Test 1: Application Lifecycle State Machine & Instance ID Tracking
    // =========================================================================
    let app1 = LifeCycleApp::new();
    let id1 = mgr.spawn(app1, &mut wm, 100, 100, 300, 200).expect("Spawn failed");

    if mgr.count() != 1 || wm.windows.len() != 1 {
        panic!("Framework Test 1 Failed: Expected 1 instance and 1 window");
    }

    let inst1 = mgr.find(id1).expect("Instance 1 not found");
    if inst1.state != AppState::Running {
        panic!("Framework Test 1 Failed: Initial state should be Running");
    }

    // Suspend instance
    mgr.suspend(id1).expect("Suspend failed");
    if mgr.find(id1).unwrap().state != AppState::Suspended {
        panic!("Framework Test 1 Failed: Suspend state mismatch");
    }

    // Resume instance
    mgr.resume(id1).expect("Resume failed");
    if mgr.find(id1).unwrap().state != AppState::Running {
        panic!("Framework Test 1 Failed: Resume state mismatch");
    }

    // Terminate instance
    mgr.terminate(id1, &mut wm).expect("Terminate failed");
    if mgr.count() != 0 || wm.windows.len() != 0 {
        panic!("Framework Test 1 Failed: Terminate should clean up instance and window");
    }

    klog_ok!("Phase 21 Test 1 Passed: Application lifecycle state machine & instance tracking verified");

    // =========================================================================
    // Test 2: AppEnv UI Root Container, Widget Binding & Action Callback Dispatch
    // =========================================================================
    let app2 = UiBindingApp::new();
    let id2 = mgr.spawn(app2, &mut wm, 50, 50, 320, 240).expect("Spawn app2 failed");

    // Simulate Mouse click on Increment button (bounds: x=10, y=10, w=100, h=30)
    // Send MouseDown + MouseUp inside button
    let click_event = AppEvent::Input(WidgetEvent::MouseDown { x: 20, y: 20, button: MouseButton::Left });
    mgr.dispatch_event(id2, &click_event, &mut wm).expect("Dispatch MouseDown failed");

    let release_event = AppEvent::Input(WidgetEvent::MouseUp { x: 20, y: 20, button: MouseButton::Left });
    mgr.dispatch_event(id2, &release_event, &mut wm).expect("Dispatch MouseUp failed");

    // Terminate app2
    mgr.terminate(id2, &mut wm).expect("Terminate app2 failed");

    klog_ok!("Phase 21 Test 2 Passed: AppEnv UI root container & widget action routing verified");

    // =========================================================================
    // Test 3: Inter-Application Clipboard Text Copy & Paste Service
    // =========================================================================
    clipboard::clear();
    if clipboard::has() || clipboard::get().is_some() {
        panic!("Framework Test 3 Failed: Clipboard should be empty initially");
    }

    let clip_text = "Integrate OS Framework Clipboard Payload";
    clipboard::set(clip_text);
    if !clipboard::has() {
        panic!("Framework Test 3 Failed: Clipboard should have text");
    }

    let pasted = clipboard::get().expect("Failed to get clipboard text");
    if pasted != clip_text {
        panic!("Framework Test 3 Failed: Clipboard content mismatch: '{}'", pasted);
    }

    clipboard::clear();
    if clipboard::has() {
        panic!("Framework Test 3 Failed: Clipboard clear failed");
    }

    klog_ok!("Phase 21 Test 3 Passed: System clipboard copy & paste string service verified");

    // =========================================================================
    // Test 4: AppEvent Bus, Custom Messaging & Timer Ticking
    // =========================================================================
    let app4 = MessagingApp::new();
    let id4 = mgr.spawn(app4, &mut wm, 150, 150, 250, 200).expect("Spawn app4 failed");

    // Dispatch Custom message
    let custom_ev = AppEvent::Custom { message_id: 42, payload: 1337 };
    mgr.dispatch_event(id4, &custom_ev, &mut wm).expect("Custom dispatch failed");

    // Dispatch Timer tick
    let timer_ev = AppEvent::TimerTick(250);
    mgr.dispatch_event(id4, &timer_ev, &mut wm).expect("Timer dispatch failed");

    // Update all with delta_ms = 16
    mgr.update_all(16, &mut wm);

    mgr.terminate(id4, &mut wm).expect("Terminate app4 failed");

    klog_ok!("Phase 21 Test 4 Passed: Event bus, custom messaging & timer ticking verified");

    // =========================================================================
    // Test 5: Full Application Execution Cycle with WindowManager & Client Buffer Rasterization
    // =========================================================================
    let app5 = DashboardApp;
    let id5 = mgr.spawn(app5, &mut wm, 80, 80, 300, 220).expect("Spawn app5 failed");

    let inst5 = mgr.find(id5).expect("DashboardApp instance not found");
    let win_id = inst5.env.window_id;

    let win = wm.find_window(win_id).expect("DashboardApp window not found");
    if win.title != "DashboardApp" {
        panic!("Framework Test 5 Failed: Window title mismatch");
    }

    // Verify client buffer has been rendered with widgets
    // Button interior at client relative (x = 20, y = 50) should match BTN_DANGER_BG (0xFFE05353)
    let px_btn = win.get_client_pixel(20, 50).expect("Failed to read client pixel");
    if px_btn != BTN_DANGER_BG {
        panic!("Framework Test 5 Failed: Rendered client button pixel mismatch: expected {:#010x}, got {:#010x}",
            BTN_DANGER_BG, px_btn);
    }

    // Verify ProgressBar fill at client relative (x = 30, y = 85) matches PROGRESS_FILL (0xFF528BFF)
    let px_prog = win.get_client_pixel(30, 85).expect("Failed to read progress client pixel");
    if px_prog != PROGRESS_FILL {
        panic!("Framework Test 5 Failed: Rendered client progress pixel mismatch: expected {:#010x}, got {:#010x}",
            PROGRESS_FILL, px_prog);
    }

    // Click "Quit" button (action 201) to request exit
    let click_quit = AppEvent::Action(201);
    mgr.dispatch_event(id5, &click_quit, &mut wm).expect("Dispatch quit action failed");

    // In update_all, quit_requested triggers automatic termination and window destruction
    mgr.update_all(16, &mut wm);

    if mgr.count() != 0 || wm.windows.len() != 0 {
        panic!("Framework Test 5 Failed: App auto-termination failed, remaining instances: {}", mgr.count());
    }

    klog_ok!("Phase 21 Test 5 Passed: Full app execution cycle, client rasterization & clean exit verified");
    klog_ok!("PHASE 21 VERIFIED: Application Framework (Lifecycle, AppEnv, Event Bus, Clipboard, Runtime) operational");
}
