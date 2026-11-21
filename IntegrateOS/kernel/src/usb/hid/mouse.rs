//! Integrate OS — USB HID Boot Protocol Mouse Driver
//!
//! Decodes standard 3-byte and 4-byte USB boot mouse input reports, accumulates
//! coordinate deltas bounded to the screen resolution, and dispatches `MouseEvent`s.

use crate::arch::x86_64::pit;
use crate::drivers::device::{DeviceCategory, Driver, MouseDriver};
use crate::drivers::traits::HardwareSupportStatus;
use crate::input::events::{InputEvent, InputEventKind, MouseButtons, MouseEvent};

/// USB HID Boot Mouse state and report parser.
pub struct UsbHidMouse {
    dev_address: u8,
    x: i32,
    y: i32,
    buttons: MouseButtons,
    screen_width: i32,
    screen_height: i32,
}

impl UsbHidMouse {
    pub const fn new(dev_address: u8) -> Self {
        Self {
            dev_address,
            x: 512,
            y: 384,
            buttons: MouseButtons::NONE,
            screen_width: 1024,
            screen_height: 768,
        }
    }

    pub fn device_address(&self) -> u8 {
        self.dev_address
    }

    /// Sets display boundaries for coordinate clamping.
    pub fn set_screen_resolution(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Processes a standard 3-byte or 4-byte USB Boot Mouse report.
    pub fn process_report(&mut self, report: &[u8]) {
        if report.len() < 3 {
            return;
        }

        let btn_byte = report[0];
        let dx = report[1] as i8 as i32;
        let dy = report[2] as i8 as i32;
        let scroll = if report.len() >= 4 {
            report[3] as i8 as i32
        } else {
            0
        };

        // Update coordinates with screen boundary clamping
        self.x = (self.x + dx).clamp(0, self.screen_width - 1);
        self.y = (self.y + dy).clamp(0, self.screen_height - 1);

        let mut buttons = MouseButtons::NONE;
        if (btn_byte & (1 << 0)) != 0 {
            buttons.0 |= MouseButtons::LEFT.0;
        }
        if (btn_byte & (1 << 1)) != 0 {
            buttons.0 |= MouseButtons::RIGHT.0;
        }
        if (btn_byte & (1 << 2)) != 0 {
            buttons.0 |= MouseButtons::MIDDLE.0;
        }

        self.buttons = buttons;

        let event = MouseEvent {
            position: (self.x, self.y),
            delta: (dx, dy),
            buttons,
            scroll,
        };

        crate::input::push_event(InputEvent::new(InputEventKind::Mouse(event), pit::get_ticks()));
    }
}

impl Driver for UsbHidMouse {
    fn name(&self) -> &str {
        "USB HID Boot Mouse"
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

impl MouseDriver for UsbHidMouse {
    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn buttons(&self) -> u8 {
        self.buttons.0
    }
}
