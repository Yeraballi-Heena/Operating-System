//! Integrate OS — Input Subsystem Architecture
//!
//! Provides:
//! - Unified input events (`events.rs`: `KeyEvent`, `MouseEvent`, `InputEvent`)
//! - Interrupt-safe event ring buffer (`queue.rs`: `InputEventQueue`)
//! - PS/2 Keyboard scancode decoder & modifier tracker (`keyboard.rs`)
//! - PS/2 Mouse driver with streaming 3-byte packets (`mouse.rs`)
//! - Virtual cursor & sprite rendering (`cursor.rs`)
//! - Input character device nodes (`vfs_node.rs`: `/dev/input/mice`, `/dev/input/event0`)
//! - Device Manager integration (`Ps2MouseDevice`)
//! - Comprehensive verification test suite (`tests.rs`)

pub mod cursor;
pub mod events;
pub mod keyboard;
pub mod mouse;
pub mod queue;
pub mod tests;
pub mod vfs_node;

use alloc::boxed::Box;

use crate::drivers::device::{DeviceCategory, Driver, MouseDriver};
use crate::drivers::manager::DEVICE_MANAGER;
use crate::drivers::traits::HardwareSupportStatus;
pub use crate::input::cursor::VirtualCursor;
pub use crate::input::events::*;
pub use crate::input::mouse::{Ps2Mouse, MOUSE};
pub use crate::input::queue::InputEventQueue;
use crate::klog_ok;
use crate::sync::IrqSpinlock;

static EVENT_QUEUE: IrqSpinlock<InputEventQueue> = IrqSpinlock::new(InputEventQueue::new());
static KEYBOARD_STATE: IrqSpinlock<keyboard::KeyboardState> =
    IrqSpinlock::new(keyboard::KeyboardState::new());

/// Pushes an input event into the central event queue.
pub fn push_event(event: InputEvent) {
    EVENT_QUEUE.lock().push(event);
}

/// Dequeues the next available input event from the central queue.
pub fn pop_event() -> Option<InputEvent> {
    EVENT_QUEUE.lock().pop()
}

/// Peeks at the next available input event without consuming it.
pub fn peek_event() -> Option<InputEvent> {
    EVENT_QUEUE.lock().peek()
}

/// Returns `true` if one or more input events are pending in the queue.
pub fn has_events() -> bool {
    !EVENT_QUEUE.lock().is_empty()
}

/// Returns the number of events currently queued.
pub fn event_count() -> usize {
    EVENT_QUEUE.lock().len()
}

/// Clears all pending events from the central event queue.
pub fn clear_events() {
    EVENT_QUEUE.lock().clear();
}

/// Returns the current screen position of the virtual mouse cursor.
pub fn cursor_position() -> (i32, i32) {
    MOUSE.lock().position()
}

/// Returns the currently active mouse button bitmask.
pub fn mouse_buttons() -> MouseButtons {
    MOUSE.lock().buttons()
}

/// Processes a raw byte from the keyboard controller and generates input events.
pub fn process_keyboard_byte(byte: u8) {
    let key_event_opt = KEYBOARD_STATE.lock().process_byte(byte);
    if let Some(key_event) = key_event_opt {
        let timestamp = crate::arch::x86_64::pit::get_ticks();
        push_event(InputEvent::new(InputEventKind::Key(key_event), timestamp));
    }
}

/// PS/2 Mouse Driver Wrapper for Device Manager integration.
pub struct Ps2MouseDevice;

impl Driver for Ps2MouseDevice {
    fn name(&self) -> &str {
        "Standard PS/2 Mouse"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Mouse
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl MouseDriver for Ps2MouseDevice {
    fn position(&self) -> (i32, i32) {
        MOUSE.lock().position()
    }

    fn buttons(&self) -> u8 {
        MOUSE.lock().buttons().0
    }
}

/// Initializes the Input Subsystem, enables PS/2 mouse streaming, registers device,
/// and mounts `/dev/input/mice` and `/dev/input/event0`.
pub fn init() {
    // 1. Initialize PS/2 mouse hardware & IRQ 12
    let _ = MOUSE.lock().init();

    // 2. Register mouse driver in central Device Manager
    let _ = DEVICE_MANAGER.lock().register(Box::new(Ps2MouseDevice));

    // 3. Mount /dev/input directory and device nodes in VFS
    let _ = crate::fs::mkdir("/dev/input");
    let _ = crate::fs::mount_node("/dev/input", Box::new(vfs_node::DevMice));
    let _ = crate::fs::mount_node("/dev/input", Box::new(vfs_node::DevEvent0));

    klog_ok!("Input Subsystem Initialized: PS/2 Keyboard & Mouse active, /dev/input mounted");
}
