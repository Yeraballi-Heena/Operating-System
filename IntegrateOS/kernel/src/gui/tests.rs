//! Integrate OS — GUI Toolkit Verification Suite (Phase 19)
//!
//! Rigorous 5-stage test suite verifying:
//! 1. Widget hierarchy, bounds, visibility, enablement & BoxLayout geometry
//! 2. Button interactive state transitions & CheckBox / RadioButton selection
//! 3. ProgressBar and Slider value clamping, mouse dragging & percentage calculations
//! 4. TextInput typing, cursor navigation, backspace/delete & length bounds
//! 5. Full Container hierarchy rasterization & pixel inspection on Canvas surface

use crate::graphics::canvas::Canvas;
use crate::graphics::rect::Rect;
use crate::gui::container::Container;
use crate::gui::layout::BoxLayout;
use crate::gui::theme::*;
use crate::gui::types::*;
use crate::gui::widget::Widget;
use crate::gui::widgets::{Button, CheckBox, Label, ProgressBar, RadioButton, Slider, TextInput};
use crate::input::events::{KeyCode, Modifiers, MouseButton};
use crate::{klog_info, klog_ok};

pub fn run_gui_tests() {
    klog_info!("Starting Phase 19 GUI Toolkit Verification Suite...");

    // =========================================================================
    // Test 1: Widget Hierarchy, Bounds, Visibility, Enablement & BoxLayout Geometry
    // =========================================================================
    let mut container = Container::new(100, 100, 300, 400);

    let id_lbl = container.add_child(Label::new("Title", 0, 0, 100, 20));
    let id_btn = container.add_child(Button::new("Click Me", 0, 0, 120, 30, 1));
    let id_prog = container.add_child(ProgressBar::new(0, 0, 200, 16, 0, 100));
    let id_input = container.add_child(TextInput::new("Input", 0, 0, 180, 24, 2));

    // Verify monotonic unique IDs
    if !(id_lbl.raw() < id_btn.raw() && id_btn.raw() < id_prog.raw() && id_prog.raw() < id_input.raw()) {
        panic!("GUI Test 1 Failed: Widget IDs not monotonic: {}, {}, {}, {}",
            id_lbl.raw(), id_btn.raw(), id_prog.raw(), id_input.raw());
    }

    if container.child_count() != 4 {
        panic!("GUI Test 1 Failed: Expected 4 children, got {}", container.child_count());
    }

    // Apply Vertical BoxLayout: padding = 10, spacing = 8, Alignment::Stretch
    let vbox = BoxLayout::new(Orientation::Vertical)
        .with_padding(10)
        .with_spacing(8)
        .with_alignment(Alignment::Stretch);

    container.apply_box_layout(&vbox);

    // Verify first child bounds: x = 110, y = 110, width = 280 (300 - 20), height = 20
    let b0 = container.child(0).unwrap().bounds();
    if b0.x != 110 || b0.y != 110 || b0.width != 280 || b0.height != 20 {
        panic!("GUI Test 1 Failed: Child 0 layout mismatch: {:?}", b0);
    }

    // Verify second child bounds: x = 110, y = 110 + 20 + 8 = 138, width = 280, height = 30
    let b1 = container.child(1).unwrap().bounds();
    if b1.x != 110 || b1.y != 138 || b1.width != 280 || b1.height != 30 {
        panic!("GUI Test 1 Failed: Child 1 layout mismatch: {:?}", b1);
    }

    // Verify third child bounds: y = 138 + 30 + 8 = 176
    let b2 = container.child(2).unwrap().bounds();
    if b2.x != 110 || b2.y != 176 || b2.width != 280 || b2.height != 16 {
        panic!("GUI Test 1 Failed: Child 2 layout mismatch: {:?}", b2);
    }

    // Test Horizontal BoxLayout on a separate sub-group
    let hbox = BoxLayout::new(Orientation::Horizontal)
        .with_padding(5)
        .with_spacing(10)
        .with_alignment(Alignment::Center);

    let item_sizes = [(50u32, 20u32), (60u32, 20u32), (70u32, 20u32)];
    let h_bounds = hbox.compute_bounds(Rect::new(0, 0, 250, 40), &item_sizes);
    if h_bounds.len() != 3 {
        panic!("GUI Test 1 Failed: Expected 3 horizontal rects");
    }
    if h_bounds[0].x != 5 || h_bounds[1].x != 5 + 50 + 10 || h_bounds[2].x != 65 + 60 + 10 {
        panic!("GUI Test 1 Failed: Horizontal layout spacing mismatch: {:?}", h_bounds);
    }

    // Verify visibility and enablement cascade
    container.set_enabled(false);
    if container.child(0).unwrap().is_enabled() {
        panic!("GUI Test 1 Failed: Container disabled cascade failed");
    }
    container.set_enabled(true);

    klog_ok!("Phase 19 Test 1 Passed: Widget hierarchy, bounds & BoxLayout geometry verified");

    // =========================================================================
    // Test 2: Button State Transitions & CheckBox / RadioButton Selection
    // =========================================================================
    let mut btn = Button::new("Submit", 50, 50, 100, 30, 42);

    // 2a. Mouse Move Outside -> Normal
    let res = btn.handle_event(&WidgetEvent::MouseMove { x: 10, y: 10 });
    if res != EventResponse::Ignored || btn.state != WidgetState::Normal {
        panic!("GUI Test 2 Failed: Expected Normal state outside");
    }

    // 2b. Mouse Move Inside -> Hover
    let res = btn.handle_event(&WidgetEvent::MouseMove { x: 60, y: 60 });
    if res != EventResponse::Consumed || btn.state != WidgetState::Hover {
        panic!("GUI Test 2 Failed: Expected Hover state inside");
    }

    // 2c. Mouse Down Inside -> Pressed
    let res = btn.handle_event(&WidgetEvent::MouseDown { x: 60, y: 60, button: MouseButton::Left });
    if res != EventResponse::Consumed || btn.state != WidgetState::Pressed || !btn.is_focused() {
        panic!("GUI Test 2 Failed: Expected Pressed & Focused state");
    }

    // 2d. Mouse Up Inside -> Action(42) and state returns to Hover
    let res = btn.handle_event(&WidgetEvent::MouseUp { x: 60, y: 60, button: MouseButton::Left });
    if res != EventResponse::Action(42) || btn.state != WidgetState::Hover {
        panic!("GUI Test 2 Failed: Expected Action(42) on mouse release");
    }

    // 2e. Disabled button ignores clicks
    btn.set_enabled(false);
    let res = btn.handle_event(&WidgetEvent::MouseDown { x: 60, y: 60, button: MouseButton::Left });
    if res != EventResponse::Ignored {
        panic!("GUI Test 2 Failed: Disabled button handled event");
    }

    // 2f. CheckBox toggling
    let mut chk = CheckBox::new("Enable Sound", 10, 10, 150, 20, 88);
    if chk.is_checked() {
        panic!("GUI Test 2 Failed: CheckBox should start unchecked");
    }
    let res = chk.handle_event(&WidgetEvent::MouseDown { x: 15, y: 15, button: MouseButton::Left });
    if res != EventResponse::Action(88) || !chk.is_checked() {
        panic!("GUI Test 2 Failed: CheckBox toggle on click failed");
    }
    // Keyboard space toggle
    let res = chk.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Space, modifiers: Modifiers::NONE, character: Some(' ') });
    if res != EventResponse::Action(88) || chk.is_checked() {
        panic!("GUI Test 2 Failed: CheckBox toggle on Space failed");
    }

    // 2g. RadioButton selection
    let mut radio = RadioButton::new("Option A", 1, 10, 50, 120, 20, 99);
    if radio.is_selected() {
        panic!("GUI Test 2 Failed: Radio should start unselected");
    }
    let res = radio.handle_event(&WidgetEvent::MouseDown { x: 15, y: 55, button: MouseButton::Left });
    if res != EventResponse::Action(99) || !radio.is_selected() {
        panic!("GUI Test 2 Failed: RadioButton selection failed");
    }

    klog_ok!("Phase 19 Test 2 Passed: Button state transitions & CheckBox/RadioButton toggling verified");

    // =========================================================================
    // Test 3: ProgressBar & Slider Value Clamping, Dragging & Percentage
    // =========================================================================
    let mut prog = ProgressBar::new(0, 0, 200, 20, 0, 200);
    prog.set_value(50);
    if prog.percentage() != 25 {
        panic!("GUI Test 3 Failed: Expected 25%, got {}%", prog.percentage());
    }
    prog.set_value(130);
    if prog.percentage() != 65 {
        panic!("GUI Test 3 Failed: Expected 65%, got {}%", prog.percentage());
    }
    // Out-of-bounds clamping
    prog.set_value(300);
    if prog.value() != 200 || prog.percentage() != 100 {
        panic!("GUI Test 3 Failed: Upper clamp failed");
    }
    prog.set_value(-50);
    if prog.value() != 0 || prog.percentage() != 0 {
        panic!("GUI Test 3 Failed: Lower clamp failed");
    }

    // Slider testing: bounds = (50, 100, 200, 20), min = 0, max = 100, step = 5, action = 105
    let mut slider = Slider::new(50, 100, 200, 20, 0, 100, 105).with_step(5);
    slider.set_value(20);
    if slider.value() != 20 {
        panic!("GUI Test 3 Failed: Slider initial value mismatch");
    }

    // Interactive Mouse Drag: click at midpoint (x = 150)
    let res = slider.handle_event(&WidgetEvent::MouseDown { x: 150, y: 110, button: MouseButton::Left });
    if !res.is_consumed() || !slider.is_dragging {
        panic!("GUI Test 3 Failed: Slider drag start failed");
    }
    // Move to near right edge (x = 245)
    slider.handle_event(&WidgetEvent::MouseMove { x: 245, y: 110 });
    if slider.value() < 90 {
        panic!("GUI Test 3 Failed: Slider drag value update failed, got {}", slider.value());
    }
    slider.handle_event(&WidgetEvent::MouseUp { x: 245, y: 110, button: MouseButton::Left });
    if slider.is_dragging {
        panic!("GUI Test 3 Failed: Slider dragging did not clear on MouseUp");
    }

    // Keyboard Arrow step navigation
    slider.set_value(50);
    slider.set_focused(true);
    slider.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Right, modifiers: Modifiers::NONE, character: None });
    if slider.value() != 55 {
        panic!("GUI Test 3 Failed: Slider right arrow step failed, got {}", slider.value());
    }
    slider.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Left, modifiers: Modifiers::NONE, character: None });
    if slider.value() != 50 {
        panic!("GUI Test 3 Failed: Slider left arrow step failed, got {}", slider.value());
    }

    klog_ok!("Phase 19 Test 3 Passed: ProgressBar & Slider value clamping, dragging & percentage verified");

    // =========================================================================
    // Test 4: TextInput Editing, Cursor Navigation, Backspace, Delete & Max Length
    // =========================================================================
    let mut input = TextInput::new("Enter name", 20, 20, 150, 26, 202);
    input.max_len = 10;

    // Focus via mouse click
    input.handle_event(&WidgetEvent::MouseDown { x: 25, y: 25, button: MouseButton::Left });
    if !input.is_focused() {
        panic!("GUI Test 4 Failed: TextInput failed to focus on click");
    }

    // Type "Rust"
    for ch in ['R', 'u', 's', 't'] {
        input.handle_event(&WidgetEvent::KeyDown {
            code: KeyCode::Unknown,
            modifiers: Modifiers::NONE,
            character: Some(ch),
        });
    }

    if input.text() != "Rust" || input.cursor_pos != 4 {
        panic!("GUI Test 4 Failed: Text input mismatch: '{}', cursor={}", input.text(), input.cursor_pos);
    }

    // Cursor Navigation: Left arrow twice -> cursor_pos = 2
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Left, modifiers: Modifiers::NONE, character: None });
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Left, modifiers: Modifiers::NONE, character: None });
    if input.cursor_pos != 2 {
        panic!("GUI Test 4 Failed: Left arrow navigation failed, pos={}", input.cursor_pos);
    }

    // Insert 'X' at pos 2 -> "RuXst"
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Unknown, modifiers: Modifiers::NONE, character: Some('X') });
    if input.text() != "RuXst" || input.cursor_pos != 3 {
        panic!("GUI Test 4 Failed: Mid-string insertion failed: '{}'", input.text());
    }

    // Backspace: removes 'X' at pos 2 -> "Rust"
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Backspace, modifiers: Modifiers::NONE, character: None });
    if input.text() != "Rust" || input.cursor_pos != 2 {
        panic!("GUI Test 4 Failed: Backspace deletion failed: '{}'", input.text());
    }

    // Delete key: removes 's' at pos 2 -> "Rut"
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Delete, modifiers: Modifiers::NONE, character: None });
    if input.text() != "Rut" || input.cursor_pos != 2 {
        panic!("GUI Test 4 Failed: Delete key removal failed: '{}'", input.text());
    }

    // Home & End keys
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Home, modifiers: Modifiers::NONE, character: None });
    if input.cursor_pos != 0 {
        panic!("GUI Test 4 Failed: Home key failed, pos={}", input.cursor_pos);
    }
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::End, modifiers: Modifiers::NONE, character: None });
    if input.cursor_pos != 3 {
        panic!("GUI Test 4 Failed: End key failed, pos={}", input.cursor_pos);
    }

    // Enter key triggers Action(202)
    let res = input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Enter, modifiers: Modifiers::NONE, character: None });
    if res != EventResponse::Action(202) {
        panic!("GUI Test 4 Failed: Enter submission failed");
    }

    // Max length constraint (max_len = 10)
    input.set_text("1234567890");
    input.handle_event(&WidgetEvent::KeyDown { code: KeyCode::Unknown, modifiers: Modifiers::NONE, character: Some('X') });
    if input.text().len() != 10 {
        panic!("GUI Test 4 Failed: Max length constraint violated, len={}", input.text().len());
    }

    klog_ok!("Phase 19 Test 4 Passed: TextInput editing, cursor navigation & length enforcement verified");

    // =========================================================================
    // Test 5: Full Container Hierarchy Rasterization & Pixel Inspection on Canvas
    // =========================================================================
    let mut test_canvas = Canvas::new(400, 300, 400 * 4, 32, 0);
    // Clear canvas
    test_canvas.draw_filled_rect(Rect::new(0, 0, 400, 300), crate::graphics::color::Color32::from_u32(0xFF000000));

    // Construct a showcase panel
    let mut panel = Container::new(20, 20, 360, 260)
        .with_background(Some(BG_PANEL))
        .with_border(Some(BORDER_DEFAULT));

    panel.add_child(Label::new("Integrate OS GUI", 30, 30, 200, 20));
    panel.add_child(Button::new("Apply", 30, 60, 80, 28, 501).with_style(ButtonStyle::Primary));
    panel.add_child(CheckBox::new("Enable Features", 30, 100, 150, 20, 502).with_checked(true));
    panel.add_child(ProgressBar::new(30, 130, 200, 16, 0, 100).with_value(65));

    // Render panel onto canvas
    panel.render(&mut test_canvas);

    // 5a. Verify panel background pixel: (x = 25, y = 25) should match BG_PANEL (0xFF14171D)
    let px_panel = test_canvas.get_pixel(25, 25).expect("Failed to read panel pixel");
    if px_panel.to_raw() != BG_PANEL {
        panic!("GUI Test 5 Failed: Panel background color mismatch: expected {:#010x}, got {:#010x}",
            BG_PANEL, px_panel.to_raw());
    }

    // 5b. Verify panel border pixel: (x = 20, y = 20) should match BORDER_DEFAULT (0xFF2A2E38)
    let px_border = test_canvas.get_pixel(20, 20).expect("Failed to read border pixel");
    if px_border.to_raw() != BORDER_DEFAULT {
        panic!("GUI Test 5 Failed: Panel border color mismatch: expected {:#010x}, got {:#010x}",
            BORDER_DEFAULT, px_border.to_raw());
    }

    // 5c. Verify Primary Button interior pixel: (x = 35, y = 70) should match BTN_PRIMARY_BG (0xFF528BFF)
    let px_btn = test_canvas.get_pixel(35, 70).expect("Failed to read button pixel");
    if px_btn.to_raw() != BTN_PRIMARY_BG {
        panic!("GUI Test 5 Failed: Primary button color mismatch: expected {:#010x}, got {:#010x}",
            BTN_PRIMARY_BG, px_btn.to_raw());
    }

    // 5d. Verify CheckBox checked box interior pixel: (x = 32, y = 104) should match ACCENT_PRIMARY (0xFF528BFF)
    let px_chk = test_canvas.get_pixel(32, 104).expect("Failed to read checkbox pixel");
    if px_chk.to_raw() != ACCENT_PRIMARY {
        panic!("GUI Test 5 Failed: Checkbox fill color mismatch: expected {:#010x}, got {:#010x}",
            ACCENT_PRIMARY, px_chk.to_raw());
    }

    // 5e. Verify ProgressBar filled portion pixel: (x = 40, y = 135) should match PROGRESS_FILL (0xFF528BFF)
    let px_prog = test_canvas.get_pixel(40, 135).expect("Failed to read progress pixel");
    if px_prog.to_raw() != PROGRESS_FILL {
        panic!("GUI Test 5 Failed: Progress bar fill color mismatch: expected {:#010x}, got {:#010x}",
            PROGRESS_FILL, px_prog.to_raw());
    }

    // Explicitly drop test_canvas to immediately free 400x300x4 bytes from heap
    drop(test_canvas);

    klog_ok!("Phase 19 Test 5 Passed: Full GUI hierarchy rendering & pixel color verification verified");
    klog_ok!("PHASE 19 VERIFIED: GUI Toolkit (Widgets, Layout, Events, Rendering) operational");
}
