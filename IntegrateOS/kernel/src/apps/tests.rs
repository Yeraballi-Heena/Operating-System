//! Integrate OS — Desktop Applications Verification Test Suite
//!
//! Validates:
//! 1. TerminalApp command execution, line editing, and canvas rasterization
//! 2. FileManagerApp directory navigation, path manipulation, and item selection
//! 3. SystemMonitorApp hardware telemetry sampling, gauges, and periodic updates
//! 4. TextEditorApp multi-line buffer editing, clipboard copy/paste, and VFS file persistence
//! 5. SettingsApp control panel interaction and multi-app orchestration in AppManager

use alloc::string::String;
use crate::apps::editor::TextEditorApp;
use crate::apps::file_manager::FileManagerApp;
use crate::apps::settings::SettingsApp;
use crate::apps::sysmon::SystemMonitorApp;
use crate::apps::terminal::TerminalApp;
use crate::apps::spawn_by_id;
use crate::desktop::types::AppId;
use crate::framework::app::Application;
use crate::framework::clipboard;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::manager::AppManager;
use crate::framework::types::InstanceId;
use crate::graphics::canvas::Canvas;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, Modifiers, MouseButton};
use crate::wm::types::WindowId;
use crate::wm::WindowManager;
use crate::{klog_info, klog_ok};

/// Executes the full Phase 22 desktop applications test suite.
pub fn run_apps_tests() {
    klog_info!("[apps-test] Starting Phase 22 Desktop Applications verification...");

    test_terminal_app();
    test_file_manager_app();
    test_system_monitor_app();
    test_text_editor_app();
    test_settings_and_multi_app();

    klog_ok!("[apps-test] ALL PHASE 22 DESKTOP APPLICATION TESTS PASSED (5/5)");
}

/// Test 1: Terminal Application command parsing, history logging, and prompt editing.
fn test_terminal_app() {
    klog_info!("[apps-test] Test 1: TerminalApp execution and shell commands...");

    let mut term = TerminalApp::new();
    let mut env = AppEnv::new(InstanceId(1), WindowId(1), 0, 560, 380);

    term.on_init(&mut env).expect("Terminal on_init failed");
    assert!(!term.history.is_empty(), "Terminal history should have banner");

    // Command: uname
    term.execute_command("uname", &mut env);
    let last = term.history.last().expect("Missing uname output");
    assert!(last.contains("IntegrateOS"), "uname output mismatch: {}", last);

    // Command: echo
    term.execute_command("echo hello terminal test", &mut env);
    let last = term.history.last().expect("Missing echo output");
    assert_eq!(last, "hello terminal test", "echo mismatch");

    // Command: uptime
    term.execute_command("uptime", &mut env);
    let last = term.history.last().expect("Missing uptime output");
    assert!(last.contains("Uptime:"), "uptime mismatch");

    // Command: mem
    term.execute_command("mem", &mut env);
    assert!(term.history.len() >= 6, "Expected memory query output in history");

    // Interactive Key input: type 'l', 's', Enter
    term.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::KeyDown {
            code: KeyCode::L,
            character: Some('l'),
            modifiers: Modifiers::default(),
        }),
    );
    term.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::KeyDown {
            code: KeyCode::S,
            character: Some('s'),
            modifiers: Modifiers::default(),
        }),
    );
    assert_eq!(term.input_line, "ls", "Input line mismatch");

    term.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::KeyDown {
            code: KeyCode::Enter,
            character: None,
            modifiers: Modifiers::default(),
        }),
    );
    assert!(term.input_line.is_empty(), "Input line should be cleared after Enter");

    // Test rasterization
    let mut canvas = Canvas::new(560, 380, 560 * 4, 32, 0);
    term.render(&mut canvas);
    let non_zero = canvas.back_buffer.iter().any(|&p| p != 0);
    assert!(non_zero, "Terminal canvas rendering produced blank output");

    klog_ok!("[apps-test] Test 1 PASSED: TerminalApp commands and line editing verified");
}

/// Test 2: File Manager Application directory navigation and VFS inspection.
fn test_file_manager_app() {
    klog_info!("[apps-test] Test 2: FileManagerApp VFS browsing and path traversal...");

    let mut fm = FileManagerApp::new();
    let mut env = AppEnv::new(InstanceId(2), WindowId(2), 0, 520, 360);

    fm.on_init(&mut env).expect("FileManager on_init failed");
    assert_eq!(fm.current_path, "/", "FileManager should start at root");
    assert!(!fm.entries.is_empty(), "Root directory entries should not be empty");

    // Navigate to /dev
    fm.navigate_to("/dev");
    assert_eq!(fm.current_path, "/dev", "Path should update to /dev");
    assert!(!fm.entries.is_empty(), "/dev should contain device nodes");

    // Navigate up back to /
    fm.navigate_up();
    assert_eq!(fm.current_path, "/", "Navigate up should return to /");

    // Select first entry
    fm.selected_index = Some(0);
    fm.open_selected();
    assert!(!fm.status_msg.is_empty(), "Status message should be set");

    // Test canvas rendering
    let mut canvas = Canvas::new(520, 360, 520 * 4, 32, 0);
    fm.render(&mut canvas);
    let non_zero = canvas.back_buffer.iter().any(|&p| p != 0);
    assert!(non_zero, "FileManager canvas rendering produced blank output");

    klog_ok!("[apps-test] Test 2 PASSED: FileManagerApp VFS navigation verified");
}

/// Test 3: System Monitor Application hardware telemetry and resource gauges.
fn test_system_monitor_app() {
    klog_info!("[apps-test] Test 3: SystemMonitorApp live metrics and resource gauges...");

    let mut sysmon = SystemMonitorApp::new();
    let mut env = AppEnv::new(InstanceId(3), WindowId(3), 0, 500, 340);

    sysmon.on_init(&mut env).expect("SystemMonitor on_init failed");
    assert!(sysmon.ram_total_kb > 0, "RAM total must be > 0");
    assert!(sysmon.ram_pct <= 100, "RAM percentage out of range");
    assert!(sysmon.heap_total_kb > 0, "Heap total must be > 0");
    assert!(sysmon.heap_pct <= 100, "Heap percentage out of range");
    assert!(sysmon.process_count >= 1, "Process count must be at least 1");

    // Periodic update
    sysmon.on_update(&mut env, 600);
    assert!(env.redraw_requested, "Redraw should be requested after 500ms");

    // Tab toggle to System Info
    sysmon.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::MouseDown {
            x: 150,
            y: 18,
            button: MouseButton::Left,
        }),
    );
    assert_eq!(sysmon.active_tab, 1, "Active tab should be System Info");

    // Canvas render
    let mut canvas = Canvas::new(500, 340, 500 * 4, 32, 0);
    sysmon.render(&mut canvas);
    let non_zero = canvas.back_buffer.iter().any(|&p| p != 0);
    assert!(non_zero, "SystemMonitor canvas rendering produced blank output");

    klog_ok!("[apps-test] Test 3 PASSED: SystemMonitorApp telemetry and gauges verified");
}

/// Test 4: Text Editor Application multi-line editing, clipboard copy/paste, and VFS file I/O.
fn test_text_editor_app() {
    klog_info!("[apps-test] Test 4: TextEditorApp multi-line editing, clipboard & VFS persistence...");

    let mut editor = TextEditorApp::new();
    let mut env = AppEnv::new(InstanceId(4), WindowId(4), 0, 540, 360);

    editor.on_init(&mut env).expect("TextEditor on_init failed");
    assert!(!editor.lines.is_empty(), "Editor initial lines empty");

    // 1. Text input & insertion
    let initial_line_count = editor.lines.len();
    editor.insert_char('A');
    assert!(editor.lines[0].contains('A'), "Character 'A' not inserted");
    assert!(editor.is_dirty, "Editor should be marked dirty after edit");

    // 2. Enter key line split
    editor.handle_enter();
    assert_eq!(editor.lines.len(), initial_line_count + 1, "Enter should increment line count");

    // 3. Backspace line merge
    editor.handle_backspace();
    assert_eq!(editor.lines.len(), initial_line_count, "Backspace should merge line back");

    // 4. System Clipboard integration
    editor.lines[0] = String::from("CLIPBOARD_INTEGRATION_TEST_LINE");
    editor.cursor_row = 0;
    editor.copy_current_line();
    let clip_text = clipboard::get().expect("Clipboard should contain copied text");
    assert_eq!(clip_text, "CLIPBOARD_INTEGRATION_TEST_LINE", "Clipboard copy mismatch");

    // Paste clipboard into line 1
    editor.cursor_row = 1;
    editor.cursor_col = 0;
    editor.paste_clipboard();
    assert!(editor.lines[1].starts_with("CLIPBOARD_INTEGRATION_TEST_LINE"), "Paste failed");

    // 5. VFS File Save and Reload
    editor.file_path = String::from("/test_editor_save.txt");
    editor.save_file().expect("Save file to VFS failed");
    assert!(!editor.is_dirty, "Dirty flag should be cleared after save");

    // Reload with a fresh editor instance
    let mut reloaded = TextEditorApp::new();
    reloaded.load_file("/test_editor_save.txt").expect("Load file from VFS failed");
    assert_eq!(reloaded.lines.len(), editor.lines.len(), "Reloaded line count mismatch");
    assert_eq!(reloaded.lines[0], editor.lines[0], "Reloaded line 0 mismatch");

    // Canvas render
    let mut canvas = Canvas::new(540, 360, 540 * 4, 32, 0);
    editor.render(&mut canvas);
    let non_zero = canvas.back_buffer.iter().any(|&p| p != 0);
    assert!(non_zero, "TextEditor canvas rendering produced blank output");

    klog_ok!("[apps-test] Test 4 PASSED: TextEditorApp multi-line editing, clipboard & VFS persistence verified");
}

/// Test 5: Settings Application interaction and AppManager multi-app lifecycle orchestration.
fn test_settings_and_multi_app() {
    klog_info!("[apps-test] Test 5: SettingsApp interaction and AppManager multi-app orchestration...");

    let mut settings = SettingsApp::new();
    let mut env = AppEnv::new(InstanceId(5), WindowId(5), 0, 480, 320);

    settings.on_init(&mut env).expect("Settings on_init failed");
    assert_eq!(settings.theme_index, 0, "Default theme index should be 0");
    assert_eq!(settings.mouse_sensitivity, 5, "Default sensitivity should be 5");
    assert!(settings.drop_shadows, "Default drop shadows should be true");

    // Click Theme 1 (Slate Blue) at y = 95
    settings.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::MouseDown {
            x: 50,
            y: 95,
            button: MouseButton::Left,
        }),
    );
    assert_eq!(settings.theme_index, 1, "Theme should be updated to 1");

    // Click Drop Shadows Checkbox at y = 225
    settings.on_event(
        &mut env,
        &AppEvent::Input(WidgetEvent::MouseDown {
            x: 30,
            y: 225,
            button: MouseButton::Left,
        }),
    );
    assert!(!settings.drop_shadows, "Drop shadows should be toggled to false");

    // Reset Defaults
    settings.reset_defaults();
    assert_eq!(settings.theme_index, 0, "Theme should reset to 0");
    assert!(settings.drop_shadows, "Drop shadows should reset to true");

    // Multi-App Orchestration in AppManager
    let mut wm = WindowManager::new(1024, 768);
    let mut app_mgr = AppManager::new();

    // Spawn all 5 apps
    let id_term = spawn_by_id(AppId::Terminal, &mut app_mgr, &mut wm).expect("Spawn Terminal failed");
    let id_fm = spawn_by_id(AppId::FileManager, &mut app_mgr, &mut wm).expect("Spawn FileManager failed");
    let id_sys = spawn_by_id(AppId::SystemMonitor, &mut app_mgr, &mut wm).expect("Spawn SysMon failed");
    let id_edit = spawn_by_id(AppId::TextEditor, &mut app_mgr, &mut wm).expect("Spawn TextEditor failed");
    let id_set = spawn_by_id(AppId::Settings, &mut app_mgr, &mut wm).expect("Spawn Settings failed");

    assert_eq!(app_mgr.count(), 5, "AppManager should have 5 running instances");
    assert_eq!(wm.windows.len(), 5, "WindowManager should have 5 active windows");

    // Verify all windows have rendered client content
    for inst in &app_mgr.instances {
        let win = wm.find_window(inst.env.window_id).expect("Window missing");
        let non_zero = win.content_buffer.iter().any(|&p| p != 0);
        assert!(non_zero, "Spawned window content buffer is completely blank");
    }

    // Terminate all instances
    app_mgr.terminate(id_term, &mut wm).expect("Terminate Terminal failed");
    app_mgr.terminate(id_fm, &mut wm).expect("Terminate FileManager failed");
    app_mgr.terminate(id_sys, &mut wm).expect("Terminate SysMon failed");
    app_mgr.terminate(id_edit, &mut wm).expect("Terminate TextEditor failed");
    app_mgr.terminate(id_set, &mut wm).expect("Terminate Settings failed");

    assert_eq!(app_mgr.count(), 0, "AppManager instances should be 0");
    assert_eq!(wm.windows.len(), 0, "WindowManager windows should be 0");

    klog_ok!("[apps-test] Test 5 PASSED: SettingsApp and AppManager multi-app lifecycle verified");
}
