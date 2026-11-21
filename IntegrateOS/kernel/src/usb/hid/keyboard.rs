//! Integrate OS — USB HID Boot Protocol Keyboard Driver
//!
//! Decodes standard 8-byte USB boot keyboard input reports, manages modifier key states,
//! dispatches `KeyEvent`s to the system input queue, and pushes ASCII characters into
//! the kernel keyboard queue.

use crate::arch::x86_64::keyboard::KEYBOARD_QUEUE;
use crate::arch::x86_64::pit;
use crate::drivers::device::{DeviceCategory, Driver, KeyboardDriver};
use crate::drivers::traits::HardwareSupportStatus;
use crate::input::events::{InputEvent, InputEventKind, KeyAction, KeyCode, KeyEvent, Modifiers};
use super::usage::*;

/// USB HID Boot Keyboard state and report parser.
pub struct UsbHidKeyboard {
    dev_address: u8,
    last_report: [u8; 8],
    modifiers: Modifiers,
    last_char: Option<u8>,
}

impl UsbHidKeyboard {
    pub const fn new(dev_address: u8) -> Self {
        Self {
            dev_address,
            last_report: [0; 8],
            modifiers: Modifiers::NONE,
            last_char: None,
        }
    }

    pub fn device_address(&self) -> u8 {
        self.dev_address
    }

    /// Processes an 8-byte USB Boot Keyboard input report.
    pub fn process_report(&mut self, report: &[u8]) {
        if report.len() < 8 {
            return;
        }

        let mod_byte = report[0];

        // Update Modifiers
        let mut mods = Modifiers::NONE;
        if (mod_byte & (1 << 0)) != 0 || (mod_byte & (1 << 4)) != 0 {
            mods.0 |= Modifiers::CTRL.0;
        }
        if (mod_byte & (1 << 1)) != 0 || (mod_byte & (1 << 5)) != 0 {
            mods.0 |= Modifiers::SHIFT.0;
        }
        if (mod_byte & (1 << 2)) != 0 || (mod_byte & (1 << 6)) != 0 {
            mods.0 |= Modifiers::ALT.0;
        }
        if (mod_byte & (1 << 3)) != 0 || (mod_byte & (1 << 7)) != 0 {
            mods.0 |= Modifiers::SUPER.0;
        }
        self.modifiers = mods;

        let prev_keys = &self.last_report[2..8];
        let curr_keys = &report[2..8];

        // Check for released keys
        for &prev in prev_keys {
            if prev != 0 && !curr_keys.contains(&prev) {
                let code = hid_usage_to_keycode(prev);
                if code != KeyCode::Unknown {
                    let event = KeyEvent {
                        code,
                        action: KeyAction::Release,
                        modifiers: self.modifiers,
                        ascii: None,
                    };
                    crate::input::push_event(InputEvent::new(InputEventKind::Key(event), pit::get_ticks()));
                }
            }
        }

        // Check for newly pressed keys
        for &curr in curr_keys {
            if curr != 0 && !prev_keys.contains(&curr) {
                let code = hid_usage_to_keycode(curr);
                let ascii = hid_usage_to_ascii(curr, self.modifiers.is_shift());

                if let Some(ch) = ascii {
                    self.last_char = Some(ch);
                    KEYBOARD_QUEUE.lock().push(ch);
                }

                if code != KeyCode::Unknown {
                    let event = KeyEvent {
                        code,
                        action: KeyAction::Press,
                        modifiers: self.modifiers,
                        ascii,
                    };
                    crate::input::push_event(InputEvent::new(InputEventKind::Key(event), pit::get_ticks()));
                }
            }
        }

        self.last_report.copy_from_slice(&report[..8]);
    }
}

impl Driver for UsbHidKeyboard {
    fn name(&self) -> &str {
        "USB HID Boot Keyboard"
    }

    fn category(&self) -> DeviceCategory {
        DeviceCategory::Keyboard
    }

    fn status(&self) -> HardwareSupportStatus {
        HardwareSupportStatus::Supported
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl KeyboardDriver for UsbHidKeyboard {
    fn read_char(&mut self) -> Option<u8> {
        self.last_char.take()
    }
}
