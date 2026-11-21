//! PS/2 Mouse Controller Driver (IRQ 12 / Vector 44)
//!
//! Handles 8042 auxiliary device initialization, streaming 3-byte packet decoding,
//! 9-bit sign-extended motion vector calculation, inverted Y coordinate mapping,
//! button state transitions, and integration with the virtual screen cursor.

use crate::arch::x86_64::interrupts::{IRQ_CASCADE, IRQ_MOUSE};
use crate::arch::x86_64::pic;
use crate::input::cursor::VirtualCursor;
use crate::input::events::{
    InputEvent, InputEventKind, MouseButton, MouseButtonAction, MouseButtons, MouseEvent,
};
use crate::serial::{inb, outb};
use crate::sync::IrqSpinlock;

pub const MOUSE_DATA_PORT: u16 = 0x60;
pub const MOUSE_STATUS_PORT: u16 = 0x64;

const TIMEOUT_CYCLES: usize = 100_000;

/// Hardware PS/2 Mouse Driver and state tracker.
pub struct Ps2Mouse {
    pub cursor: VirtualCursor,
    cycle: u8,
    packet: [u8; 3],
    last_buttons: MouseButtons,
    raw_packet_history: [u8; 3],
    initialized: bool,
    pub sensitivity_enabled: bool,
}

impl Ps2Mouse {
    pub const fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            cursor: VirtualCursor::new(screen_width, screen_height),
            cycle: 0,
            packet: [0; 3],
            last_buttons: MouseButtons::NONE,
            raw_packet_history: [0; 3],
            initialized: false,
            sensitivity_enabled: false,
        }
    }

    /// Toggles desktop mouse motion scaling and acceleration.
    pub fn set_sensitivity(&mut self, enabled: bool) {
        self.sensitivity_enabled = enabled;
    }

    /// Returns the current cursor `(x, y)` coordinate.
    pub fn position(&self) -> (i32, i32) {
        self.cursor.position()
    }

    /// Returns the currently held mouse button bitmask.
    pub fn buttons(&self) -> MouseButtons {
        self.last_buttons
    }

    /// Returns the most recently received raw 3-byte hardware packet.
    pub fn last_raw_packet(&self) -> [u8; 3] {
        self.raw_packet_history
    }

    /// Waits until the 8042 controller input buffer is empty.
    fn wait_input_ready() -> Result<(), &'static str> {
        for _ in 0..TIMEOUT_CYCLES {
            // SAFETY: Reading status register from port 0x64.
            let status = unsafe { inb(MOUSE_STATUS_PORT) };
            if (status & 0x02) == 0 {
                return Ok(());
            }
        }
        Err("PS/2 controller input buffer wait timed out")
    }

    /// Waits until the 8042 controller output buffer has data to read.
    fn wait_output_ready() -> Result<(), &'static str> {
        for _ in 0..TIMEOUT_CYCLES {
            // SAFETY: Reading status register from port 0x64.
            let status = unsafe { inb(MOUSE_STATUS_PORT) };
            if (status & 0x01) != 0 {
                return Ok(());
            }
        }
        Err("PS/2 controller output buffer wait timed out")
    }

    /// Sends a command byte directly to the mouse via the 0xD4 prefix.
    pub fn write_mouse(byte: u8) -> Result<u8, &'static str> {
        Self::wait_input_ready()?;
        // SAFETY: Directing next data byte to second PS/2 port (auxiliary / mouse).
        unsafe {
            outb(MOUSE_STATUS_PORT, 0xD4);
        }
        Self::wait_input_ready()?;
        // SAFETY: Writing command byte to PS/2 data port.
        unsafe {
            outb(MOUSE_DATA_PORT, byte);
        }
        Self::wait_output_ready()?;
        // SAFETY: Reading response/acknowledgement from data port.
        let ack = unsafe { inb(MOUSE_DATA_PORT) };
        Ok(ack)
    }

    /// Initializes PS/2 mouse hardware and unmasks IRQ 12 in the PIC.
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Flush any lingering bytes from output buffer
        for _ in 0..16 {
            // SAFETY: Checking output buffer status.
            let status = unsafe { inb(MOUSE_STATUS_PORT) };
            if (status & 0x01) == 0 {
                break;
            }
            // SAFETY: Consuming lingering byte.
            unsafe {
                inb(MOUSE_DATA_PORT);
            }
        }

        // 1. Enable auxiliary PS/2 device
        Self::wait_input_ready()?;
        // SAFETY: Command 0xA8 enables second PS/2 port.
        unsafe {
            outb(MOUSE_STATUS_PORT, 0xA8);
        }

        // 2. Enable keyboard IRQ 1 and mouse IRQ 12 in controller configuration byte
        Self::wait_input_ready()?;
        // SAFETY: Command 0x20 reads controller configuration byte.
        unsafe {
            outb(MOUSE_STATUS_PORT, 0x20);
        }
        Self::wait_output_ready()?;
        let mut config = unsafe { inb(MOUSE_DATA_PORT) };
        config |= 0x03; // Enable IRQ 1 (keyboard) and IRQ 12 (auxiliary mouse)
        config &= !0x30; // Clear both keyboard and mouse clock disable bits

        Self::wait_input_ready()?;
        // SAFETY: Command 0x60 writes controller configuration byte.
        unsafe {
            outb(MOUSE_STATUS_PORT, 0x60);
        }
        Self::wait_input_ready()?;
        // SAFETY: Sending modified configuration byte.
        unsafe {
            outb(MOUSE_DATA_PORT, config);
        }

        // 3. Enable first PS/2 port (keyboard) explicitly (command 0xAE)
        Self::wait_input_ready()?;
        unsafe {
            outb(MOUSE_STATUS_PORT, 0xAE);
        }

        // 4. Set default mouse settings (command 0xF6)
        let _ = Self::write_mouse(0xF6);

        // 5. Enable mouse packet streaming data reporting (command 0xF4)
        let _ = Self::write_mouse(0xF4);

        // 6. Flush any remaining command ACK (0xFA) bytes from 8042 output buffer
        for _ in 0..16 {
            let status = unsafe { inb(MOUSE_STATUS_PORT) };
            if (status & 0x01) == 0 {
                break;
            }
            unsafe {
                inb(MOUSE_DATA_PORT);
            }
        }
        self.cycle = 0;

        // 7. Unmask IRQ 2 (Cascade) on Master PIC and IRQ 12 on Slave PIC
        pic::unmask_irq(IRQ_CASCADE);
        pic::unmask_irq(IRQ_MOUSE);

        self.initialized = true;
        Ok(())
    }

    /// Updates the virtual cursor screen boundary dimensions when display resolution changes.
    pub fn set_screen_bounds(&mut self, width: i32, height: i32) {
        let (cx, cy) = self.cursor.position();
        self.cursor = VirtualCursor::new(width, height);
        self.cursor.set_position(cx, cy);
    }

    /// Processes a single raw byte received from the mouse controller.
    ///
    /// Decodes complete 3-byte packets, updates cursor position, and emits
    /// `InputEvent`s to the central event queue.
    pub fn process_byte(&mut self, byte: u8, timestamp: u64) {
        match self.cycle {
            0 => {
                // Ignore PS/2 control/ACK bytes (0xFA, 0xFE, 0xFC) or invalid overflow header
                if byte == 0xFA || byte == 0xFE || byte == 0xFC || (byte & 0xC0) == 0xC0 {
                    return;
                }
                // Synchronization check: bit 3 of byte 0 must always be 1
                if (byte & 0x08) == 0 {
                    // Out of sync; discard byte
                    return;
                }
                self.packet[0] = byte;
                self.cycle = 1;
            }
            1 => {
                self.packet[1] = byte;
                self.cycle = 2;
            }
            2 => {
                self.packet[2] = byte;
                self.cycle = 0;
                self.raw_packet_history = self.packet;

                // Decode complete 3-byte packet
                self.decode_packet(timestamp);
            }
            _ => self.cycle = 0,
        }
    }

    /// Decodes a complete 3-byte PS/2 packet and dispatches events.
    pub fn decode_packet(&mut self, timestamp: u64) {
        let flags = self.packet[0];
        let x_raw = self.packet[1];
        let y_raw = self.packet[2];

        // Button bits
        let left = (flags & 0x01) != 0;
        let right = (flags & 0x02) != 0;
        let middle = (flags & 0x04) != 0;

        let x_sign = (flags & 0x10) != 0;
        let y_sign = (flags & 0x20) != 0;
        let x_overflow = (flags & 0x40) != 0;
        let y_overflow = (flags & 0x80) != 0;

        // Sign extension
        let dx = if x_overflow {
            0
        } else if x_sign {
            ((x_raw as u16) | 0xFF00) as i16 as i32
        } else {
            x_raw as i32
        };

        let dy_raw = if y_overflow {
            0
        } else if y_sign {
            ((y_raw as u16) | 0xFF00) as i16 as i32
        } else {
            y_raw as i32
        };

        // Invert Y delta: PS/2 positive is up, screen positive is down
        let dy = -dy_raw;

        // Apply optional desktop mouse sensitivity scaling and acceleration curve
        let (scaled_dx, scaled_dy) = if self.sensitivity_enabled {
            let scale_delta = |d: i32| -> i32 {
                let abs_d = d.abs();
                if abs_d > 6 {
                    d * 3
                } else if abs_d > 2 {
                    d * 2
                } else {
                    (d * 3) / 2
                }
            };
            (scale_delta(dx), scale_delta(dy))
        } else {
            (dx, dy)
        };

        // Update virtual cursor
        self.cursor.move_relative(scaled_dx, scaled_dy);
        let pos = self.cursor.position();

        let mut current_buttons = MouseButtons::NONE;
        if left {
            current_buttons.0 |= MouseButtons::LEFT.0;
        }
        if right {
            current_buttons.0 |= MouseButtons::RIGHT.0;
        }
        if middle {
            current_buttons.0 |= MouseButtons::MIDDLE.0;
        }

        // Emit motion event if moved
        if scaled_dx != 0 || scaled_dy != 0 {
            crate::input::push_event(InputEvent::new(
                InputEventKind::MouseMove {
                    position: pos,
                    delta: (scaled_dx, scaled_dy),
                },
                timestamp,
            ));
        }

        // Detect button state changes
        if left != self.last_buttons.is_left() {
            let action = if left {
                MouseButtonAction::Press
            } else {
                MouseButtonAction::Release
            };
            crate::input::push_event(InputEvent::new(
                InputEventKind::MouseButton {
                    button: MouseButton::Left,
                    action,
                    position: pos,
                },
                timestamp,
            ));
        }

        if right != self.last_buttons.is_right() {
            let action = if right {
                MouseButtonAction::Press
            } else {
                MouseButtonAction::Release
            };
            crate::input::push_event(InputEvent::new(
                InputEventKind::MouseButton {
                    button: MouseButton::Right,
                    action,
                    position: pos,
                },
                timestamp,
            ));
        }

        if middle != self.last_buttons.is_middle() {
            let action = if middle {
                MouseButtonAction::Press
            } else {
                MouseButtonAction::Release
            };
            crate::input::push_event(InputEvent::new(
                InputEventKind::MouseButton {
                    button: MouseButton::Middle,
                    action,
                    position: pos,
                },
                timestamp,
            ));
        }

        // Emit combined mouse state event
        crate::input::push_event(InputEvent::new(
            InputEventKind::Mouse(MouseEvent {
                position: pos,
                delta: (dx, dy),
                buttons: current_buttons,
                scroll: 0,
            }),
            timestamp,
        ));

        self.last_buttons = current_buttons;
    }
}

pub static MOUSE: IrqSpinlock<Ps2Mouse> = IrqSpinlock::new(Ps2Mouse::new(1024, 768));

/// Handles IRQ 12 hardware interrupt from PS/2 mouse.
pub fn handle_mouse_interrupt() {
    let _ = crate::arch::x86_64::keyboard::drain_8042_controller();
}
