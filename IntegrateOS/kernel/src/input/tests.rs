//! Phase 16 — Input Subsystem Verification Suite
//!
//! Validates:
//! 1. Keyboard scancode decoding, extended prefixes (`0xE0`), and modifier tracking (Shift, CapsLock)
//! 2. PS/2 mouse 3-byte packet decoding, 9-bit sign extension, inverted Y screen delta, and buttons
//! 3. Virtual cursor boundary clamping `(0..1023, 0..767)` and sprite renderer boundary clipping
//! 4. Central input event queue FIFO ordering, push/pop integrity, and overflow resilience
//! 5. VFS device nodes (`/dev/input/mice`, `/dev/input/event0`) and central Device Manager registration

use crate::drivers::device::DeviceCategory;
use crate::drivers::manager::DEVICE_MANAGER;
use crate::drivers::traits::HardwareSupportStatus;
use crate::fs::vfs::{NodeType, OpenFlags};
use crate::fs::{open, read, stat};
use crate::input::cursor::VirtualCursor;
use crate::input::events::{
    InputEvent, InputEventKind, KeyAction, KeyCode, MouseButton, MouseButtonAction,
};
use crate::input::keyboard::KeyboardState;
use crate::input::mouse::Ps2Mouse;
use crate::input::queue::InputEventQueue;
use crate::input::push_event;
use crate::{klog_info, klog_ok};

/// Executes the comprehensive Phase 16 Input Subsystem test suite.
pub fn run_input_tests() {
    klog_info!("Starting Phase 16 Input Subsystem Verification Suite...");

    // Test 1: Keyboard Scancode Decoding, Extended Scancodes & Modifiers
    let mut kbd = KeyboardState::new();

    // 1a. Standard make code for 'A' (0x1E) -> 'a'
    let ev1 = kbd.process_byte(0x1E).expect("Failed to decode scancode 0x1E");
    if ev1.code != KeyCode::A || ev1.action != KeyAction::Press || ev1.ascii != Some(b'a') {
        panic!("Input Test 1 Failed: 'A' make code mismatch");
    }

    // 1b. Left Shift press (0x2A) followed by 'A' (0x1E) -> 'A'
    kbd.process_byte(0x2A).expect("Failed to process LeftShift");
    let ev_shift_a = kbd.process_byte(0x1E).expect("Failed to decode shifted 'A'");
    if ev_shift_a.code != KeyCode::A || ev_shift_a.ascii != Some(b'A') || !ev_shift_a.modifiers.is_shift() {
        panic!("Input Test 1 Failed: Shifted 'A' mismatch");
    }

    // Left Shift release (0xAA)
    kbd.process_byte(0xAA).expect("Failed to process LeftShift release");

    // 1c. Caps Lock toggle (0x3A) followed by 'B' (0x30) -> 'B'
    kbd.process_byte(0x3A).expect("Failed to process CapsLock");
    let ev_caps_b = kbd.process_byte(0x30).expect("Failed to decode caps 'B'");
    if ev_caps_b.code != KeyCode::B || ev_caps_b.ascii != Some(b'B') || !ev_caps_b.modifiers.is_caps() {
        panic!("Input Test 1 Failed: CapsLock 'B' mismatch");
    }
    // Turn CapsLock off
    kbd.process_byte(0x3A).expect("Failed to turn off CapsLock");

    // 1d. Extended scancode: Up Arrow (0xE0, 0x48)
    assert!(kbd.process_byte(0xE0).is_none());
    let ev_up = kbd.process_byte(0x48).expect("Failed to decode Up Arrow");
    if ev_up.code != KeyCode::Up || ev_up.action != KeyAction::Press {
        panic!("Input Test 1 Failed: Extended Up Arrow mismatch");
    }

    // 1e. Extended scancode: Delete (0xE0, 0x53) make and break (0xE0, 0xD3)
    assert!(kbd.process_byte(0xE0).is_none());
    let ev_del_make = kbd.process_byte(0x53).expect("Failed to decode Delete press");
    if ev_del_make.code != KeyCode::Delete || ev_del_make.action != KeyAction::Press {
        panic!("Input Test 1 Failed: Extended Delete press mismatch");
    }
    assert!(kbd.process_byte(0xE0).is_none());
    let ev_del_break = kbd.process_byte(0xD3).expect("Failed to decode Delete release");
    if ev_del_break.code != KeyCode::Delete || ev_del_break.action != KeyAction::Release {
        panic!("Input Test 1 Failed: Extended Delete release mismatch");
    }

    klog_ok!("Phase 16 Test 1 Passed: Keyboard scancodes, extended keys & modifier states verified");

    // Test 2: PS/2 Mouse 3-Byte Packet Decoding & Vector Math
    let mut mouse = Ps2Mouse::new(1024, 768);

    // 2a. Positive movement packet: [0x08, 25, 40]
    // 0x08 = sync bit 3 set, positive X, positive Y
    mouse.process_byte(0x08, 100);
    mouse.process_byte(25, 100);
    mouse.process_byte(40, 100);
    // Initial center was (512, 384). dx = +25, dy = -40 (inverted Y) -> (537, 344)
    let pos1 = mouse.position();
    if pos1 != (537, 344) {
        panic!("Input Test 2 Failed: Expected (537, 344), got {:?}", pos1);
    }

    // 2b. Negative movement packet with sign extension: [0x38, 0xF6, 0xEC]
    // 0x38 = sync(bit 3)=1, x_sign(bit 4)=1, y_sign(bit 5)=1
    // 0xF6 = -10, 0xEC = -20
    // dx = -10, dy = -(-20) = +20
    mouse.process_byte(0x38, 101);
    mouse.process_byte(0xF6, 101);
    mouse.process_byte(0xEC, 101);
    let pos2 = mouse.position();
    // (537 - 10, 344 + 20) = (527, 364)
    if pos2 != (527, 364) {
        panic!("Input Test 2 Failed: Expected (527, 364), got {:?}", pos2);
    }

    // 2c. Button clicks: [0x09, 0, 0] = Left button pressed
    mouse.process_byte(0x09, 102);
    mouse.process_byte(0x00, 102);
    mouse.process_byte(0x00, 102);
    if !mouse.buttons().is_left() || mouse.buttons().is_right() {
        panic!("Input Test 2 Failed: Left mouse button bitmask not set");
    }

    // [0x0A, 0, 0] = Right button pressed, left released
    mouse.process_byte(0x0A, 103);
    mouse.process_byte(0x00, 103);
    mouse.process_byte(0x00, 103);
    if !mouse.buttons().is_right() || mouse.buttons().is_left() {
        panic!("Input Test 2 Failed: Right mouse button bitmask not set");
    }

    klog_ok!("Phase 16 Test 2 Passed: PS/2 mouse 3-byte packets, sign extension & button bitmasks verified");

    // Test 3: Virtual Mouse Cursor Boundary Clamping & Sprite Renderer
    let mut cursor = VirtualCursor::new(1024, 768);
    // Center is (512, 384)
    // 3a. Move off right and bottom edge
    cursor.move_relative(2000, 2000);
    if cursor.position() != (1023, 767) {
        panic!("Input Test 3 Failed: Right/bottom boundary clamping failed: {:?}", cursor.position());
    }

    // 3b. Move off top and left edge
    cursor.move_relative(-5000, -5000);
    if cursor.position() != (0, 0) {
        panic!("Input Test 3 Failed: Top/left boundary clamping failed: {:?}", cursor.position());
    }

    // 3c. Verify cursor sprite rendering within screen bounds
    cursor.set_position(1020, 760); // Near bottom-right edge where sprite partially clips
    let mut drawn_pixels = 0;
    let mut out_of_bounds = false;
    cursor.render(|px, py, _color| {
        drawn_pixels += 1;
        if px < 0 || px >= 1024 || py < 0 || py >= 768 {
            out_of_bounds = true;
        }
    });
    if out_of_bounds || drawn_pixels == 0 {
        panic!("Input Test 3 Failed: Cursor sprite rendered out-of-bounds or zero pixels");
    }
    klog_ok!("Phase 16 Test 3 Passed: Screen boundary clamping (1024x768) & sprite clipping verified");

    // Test 4: Central Input Event Queue FIFO & Overflow Resilience
    let mut queue = InputEventQueue::new();
    assert!(queue.is_empty());

    // Push 3 distinct events
    queue.push(InputEvent::new(
        InputEventKind::MouseButton {
            button: MouseButton::Left,
            action: MouseButtonAction::Press,
            position: (100, 200),
        },
        1000,
    ));
    queue.push(InputEvent::new(
        InputEventKind::MouseMove {
            position: (150, 250),
            delta: (50, 50),
        },
        1001,
    ));
    queue.push(InputEvent::new(
        InputEventKind::MouseButton {
            button: MouseButton::Left,
            action: MouseButtonAction::Release,
            position: (150, 250),
        },
        1002,
    ));

    if queue.len() != 3 {
        panic!("Input Test 4 Failed: Expected 3 queued events, got {}", queue.len());
    }

    // Verify FIFO pop order
    let popped1 = queue.pop().expect("Pop 1 failed");
    if popped1.timestamp != 1000 {
        panic!("Input Test 4 Failed: FIFO ordering mismatch on event 1");
    }
    let popped2 = queue.pop().expect("Pop 2 failed");
    if popped2.timestamp != 1001 {
        panic!("Input Test 4 Failed: FIFO ordering mismatch on event 2");
    }
    let popped3 = queue.pop().expect("Pop 3 failed");
    if popped3.timestamp != 1002 {
        panic!("Input Test 4 Failed: FIFO ordering mismatch on event 3");
    }
    assert!(queue.is_empty());

    // Test ring buffer overflow resilience (capacity is 256)
    for i in 0..260 {
        queue.push(InputEvent::new(
            InputEventKind::MouseMove {
                position: (i, i),
                delta: (1, 1),
            },
            i as u64,
        ));
    }
    if queue.len() != 256 || queue.overflow_count() != 4 {
        panic!(
            "Input Test 4 Failed: Overflow metrics incorrect: len={}, overflows={}",
            queue.len(),
            queue.overflow_count()
        );
    }
    klog_ok!("Phase 16 Test 4 Passed: Input event queue FIFO buffering & overflow resilience verified");

    // Test 5: VFS Device Nodes (/dev/input/mice, /dev/input/event0) & Device Manager
    // 5a. Stat /dev/input directory
    let input_dir_meta = stat("/dev/input").expect("Failed to stat /dev/input");
    if input_dir_meta.node_type != NodeType::Directory {
        panic!("Input Test 5 Failed: /dev/input is not a directory");
    }

    // 5b. Read /dev/input/mice (should return 3 bytes)
    let fd_mice = open("/dev/input/mice", OpenFlags::READ).expect("Failed to open /dev/input/mice");
    let mut mice_buf = [0u8; 3];
    let bytes_mice = read(fd_mice, &mut mice_buf).expect("Failed to read from /dev/input/mice");
    if bytes_mice != 3 {
        panic!("Input Test 5 Failed: /dev/input/mice did not return 3 bytes");
    }
    crate::fs::close(fd_mice).expect("Failed to close /dev/input/mice");

    // 5c. Clear queue, push a test event and read /dev/input/event0 (should return 16-byte record)
    crate::input::clear_events();
    push_event(InputEvent::new(
        InputEventKind::MouseMove {
            position: (400, 300),
            delta: (10, -5),
        },
        9999,
    ));

    let fd_ev = open("/dev/input/event0", OpenFlags::READ).expect("Failed to open /dev/input/event0");
    let mut ev_buf = [0u8; 16];
    let bytes_ev = read(fd_ev, &mut ev_buf).expect("Failed to read /dev/input/event0");
    if bytes_ev != 16 {
        panic!("Input Test 5 Failed: /dev/input/event0 did not return 16 bytes");
    }
    let read_ts = u64::from_ne_bytes(ev_buf[0..8].try_into().unwrap());
    if read_ts != 9999 {
        panic!("Input Test 5 Failed: Event timestamp mismatch: expected 9999, got {}", read_ts);
    }
    crate::fs::close(fd_ev).expect("Failed to close /dev/input/event0");

    // 5d. Verify Device Manager tracks PS/2 Mouse as Supported
    let dev_mgr = DEVICE_MANAGER.lock();
    let mouse_devs = dev_mgr.find_by_category(DeviceCategory::Mouse);
    if mouse_devs.is_empty() {
        panic!("Input Test 5 Failed: PS/2 Mouse not registered in Device Manager");
    }
    if mouse_devs[0].status() != HardwareSupportStatus::Supported {
        panic!("Input Test 5 Failed: PS/2 Mouse status is not Supported");
    }

    klog_ok!("Phase 16 Test 5 Passed: /dev/input/mice, /dev/input/event0 & Device Manager verified");
    klog_ok!("PHASE 16 VERIFIED: Input Subsystem (Keyboard, Mouse, Cursor, Events) operational");
}
