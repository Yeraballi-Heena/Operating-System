//! PS/2 Keyboard Scancode Decoder & State Tracker
//!
//! Decodes PS/2 Scancode Set 1, tracks multi-byte extended scancodes (`0xE0`),
//! maintains modifier bitmask state, and converts raw hardware events to `KeyEvent`s.

use crate::input::events::{KeyCode, KeyAction, KeyEvent, Modifiers};

/// Keyboard state machine and scancode decoder.
pub struct KeyboardState {
    extended: bool,
    shift_left: bool,
    shift_right: bool,
    ctrl_left: bool,
    ctrl_right: bool,
    alt_left: bool,
    alt_right: bool,
    super_left: bool,
    super_right: bool,
    caps_lock: bool,
    num_lock: bool,
    scroll_lock: bool,
}

impl KeyboardState {
    pub const fn new() -> Self {
        Self {
            extended: false,
            shift_left: false,
            shift_right: false,
            ctrl_left: false,
            ctrl_right: false,
            alt_left: false,
            alt_right: false,
            super_left: false,
            super_right: false,
            caps_lock: false,
            num_lock: true,
            scroll_lock: false,
        }
    }

    /// Returns current active modifier bitmask.
    pub fn current_modifiers(&self) -> Modifiers {
        let mut m = Modifiers::NONE;
        if self.shift_left || self.shift_right {
            m.0 |= Modifiers::SHIFT.0;
        }
        if self.ctrl_left || self.ctrl_right {
            m.0 |= Modifiers::CTRL.0;
        }
        if self.alt_left || self.alt_right {
            m.0 |= Modifiers::ALT.0;
        }
        if self.super_left || self.super_right {
            m.0 |= Modifiers::SUPER.0;
        }
        if self.caps_lock {
            m.0 |= Modifiers::CAPS_LOCK.0;
        }
        if self.num_lock {
            m.0 |= Modifiers::NUM_LOCK.0;
        }
        if self.scroll_lock {
            m.0 |= Modifiers::SCROLL_LOCK.0;
        }
        m
    }

    /// Processes a single raw byte from keyboard port 0x60.
    pub fn process_byte(&mut self, byte: u8) -> Option<KeyEvent> {
        // Check for extended scancode prefix
        if byte == 0xE0 {
            self.extended = true;
            return None;
        }

        let is_extended = self.extended;
        self.extended = false;

        let is_release = (byte & 0x80) != 0;
        let make_code = byte & 0x7F;
        let action = if is_release {
            KeyAction::Release
        } else {
            KeyAction::Press
        };

        let code = if is_extended {
            match make_code {
                0x1C => KeyCode::KpEnter,
                0x1D => KeyCode::RightCtrl,
                0x35 => KeyCode::KpDivide,
                0x38 => KeyCode::RightAlt,
                0x47 => KeyCode::Home,
                0x48 => KeyCode::Up,
                0x49 => KeyCode::PageUp,
                0x4B => KeyCode::Left,
                0x4D => KeyCode::Right,
                0x4F => KeyCode::End,
                0x50 => KeyCode::Down,
                0x51 => KeyCode::PageDown,
                0x52 => KeyCode::Insert,
                0x53 => KeyCode::Delete,
                0x5B => KeyCode::LeftSuper,
                0x5C => KeyCode::RightSuper,
                _ => KeyCode::Unknown,
            }
        } else {
            match make_code {
                0x01 => KeyCode::Escape,
                0x02 => KeyCode::Num1,
                0x03 => KeyCode::Num2,
                0x04 => KeyCode::Num3,
                0x05 => KeyCode::Num4,
                0x06 => KeyCode::Num5,
                0x07 => KeyCode::Num6,
                0x08 => KeyCode::Num7,
                0x09 => KeyCode::Num8,
                0x0A => KeyCode::Num9,
                0x0B => KeyCode::Num0,
                0x0C => KeyCode::Minus,
                0x0D => KeyCode::Equal,
                0x0E => KeyCode::Backspace,
                0x0F => KeyCode::Tab,
                0x10 => KeyCode::Q,
                0x11 => KeyCode::W,
                0x12 => KeyCode::E,
                0x13 => KeyCode::R,
                0x14 => KeyCode::T,
                0x15 => KeyCode::Y,
                0x16 => KeyCode::U,
                0x17 => KeyCode::I,
                0x18 => KeyCode::O,
                0x19 => KeyCode::P,
                0x1A => KeyCode::LeftBracket,
                0x1B => KeyCode::RightBracket,
                0x1C => KeyCode::Enter,
                0x1D => KeyCode::LeftCtrl,
                0x1E => KeyCode::A,
                0x1F => KeyCode::S,
                0x20 => KeyCode::D,
                0x21 => KeyCode::F,
                0x22 => KeyCode::G,
                0x23 => KeyCode::H,
                0x24 => KeyCode::J,
                0x25 => KeyCode::K,
                0x26 => KeyCode::L,
                0x27 => KeyCode::Semicolon,
                0x28 => KeyCode::Apostrophe,
                0x29 => KeyCode::Grave,
                0x2A => KeyCode::LeftShift,
                0x2B => KeyCode::Backslash,
                0x2C => KeyCode::Z,
                0x2D => KeyCode::X,
                0x2E => KeyCode::C,
                0x2F => KeyCode::V,
                0x30 => KeyCode::B,
                0x31 => KeyCode::N,
                0x32 => KeyCode::M,
                0x33 => KeyCode::Comma,
                0x34 => KeyCode::Period,
                0x35 => KeyCode::Slash,
                0x36 => KeyCode::RightShift,
                0x37 => KeyCode::KpMultiply,
                0x38 => KeyCode::LeftAlt,
                0x39 => KeyCode::Space,
                0x3A => KeyCode::CapsLock,
                0x3B => KeyCode::F1,
                0x3C => KeyCode::F2,
                0x3D => KeyCode::F3,
                0x3E => KeyCode::F4,
                0x3F => KeyCode::F5,
                0x40 => KeyCode::F6,
                0x41 => KeyCode::F7,
                0x42 => KeyCode::F8,
                0x43 => KeyCode::F9,
                0x44 => KeyCode::F10,
                0x45 => KeyCode::NumLock,
                0x46 => KeyCode::ScrollLock,
                0x57 => KeyCode::F11,
                0x58 => KeyCode::F12,
                _ => KeyCode::Unknown,
            }
        };

        // Update internal modifier states
        let is_pressed = !is_release;
        match code {
            KeyCode::LeftShift => self.shift_left = is_pressed,
            KeyCode::RightShift => self.shift_right = is_pressed,
            KeyCode::LeftCtrl => self.ctrl_left = is_pressed,
            KeyCode::RightCtrl => self.ctrl_right = is_pressed,
            KeyCode::LeftAlt => self.alt_left = is_pressed,
            KeyCode::RightAlt => self.alt_right = is_pressed,
            KeyCode::LeftSuper => self.super_left = is_pressed,
            KeyCode::RightSuper => self.super_right = is_pressed,
            KeyCode::CapsLock if is_pressed => self.caps_lock = !self.caps_lock,
            KeyCode::NumLock if is_pressed => self.num_lock = !self.num_lock,
            KeyCode::ScrollLock if is_pressed => self.scroll_lock = !self.scroll_lock,
            _ => {}
        }

        let modifiers = self.current_modifiers();
        let ascii = if is_pressed {
            Self::key_to_ascii(code, modifiers)
        } else {
            None
        };

        Some(KeyEvent {
            code,
            action,
            modifiers,
            ascii,
        })
    }

    /// Converts a key code and modifier context into an ASCII character.
    pub fn key_to_ascii(code: KeyCode, modifiers: Modifiers) -> Option<u8> {
        let is_shift = modifiers.is_shift();
        let is_caps = modifiers.is_caps();

        match code {
            KeyCode::A => Some(if is_shift ^ is_caps { b'A' } else { b'a' }),
            KeyCode::B => Some(if is_shift ^ is_caps { b'B' } else { b'b' }),
            KeyCode::C => Some(if is_shift ^ is_caps { b'C' } else { b'c' }),
            KeyCode::D => Some(if is_shift ^ is_caps { b'D' } else { b'd' }),
            KeyCode::E => Some(if is_shift ^ is_caps { b'E' } else { b'e' }),
            KeyCode::F => Some(if is_shift ^ is_caps { b'F' } else { b'f' }),
            KeyCode::G => Some(if is_shift ^ is_caps { b'G' } else { b'g' }),
            KeyCode::H => Some(if is_shift ^ is_caps { b'H' } else { b'h' }),
            KeyCode::I => Some(if is_shift ^ is_caps { b'I' } else { b'i' }),
            KeyCode::J => Some(if is_shift ^ is_caps { b'J' } else { b'j' }),
            KeyCode::K => Some(if is_shift ^ is_caps { b'K' } else { b'k' }),
            KeyCode::L => Some(if is_shift ^ is_caps { b'L' } else { b'l' }),
            KeyCode::M => Some(if is_shift ^ is_caps { b'M' } else { b'm' }),
            KeyCode::N => Some(if is_shift ^ is_caps { b'N' } else { b'n' }),
            KeyCode::O => Some(if is_shift ^ is_caps { b'O' } else { b'o' }),
            KeyCode::P => Some(if is_shift ^ is_caps { b'P' } else { b'p' }),
            KeyCode::Q => Some(if is_shift ^ is_caps { b'Q' } else { b'q' }),
            KeyCode::R => Some(if is_shift ^ is_caps { b'R' } else { b'r' }),
            KeyCode::S => Some(if is_shift ^ is_caps { b'S' } else { b's' }),
            KeyCode::T => Some(if is_shift ^ is_caps { b'T' } else { b't' }),
            KeyCode::U => Some(if is_shift ^ is_caps { b'U' } else { b'u' }),
            KeyCode::V => Some(if is_shift ^ is_caps { b'V' } else { b'v' }),
            KeyCode::W => Some(if is_shift ^ is_caps { b'W' } else { b'w' }),
            KeyCode::X => Some(if is_shift ^ is_caps { b'X' } else { b'x' }),
            KeyCode::Y => Some(if is_shift ^ is_caps { b'Y' } else { b'y' }),
            KeyCode::Z => Some(if is_shift ^ is_caps { b'Z' } else { b'z' }),

            KeyCode::Num1 => Some(if is_shift { b'!' } else { b'1' }),
            KeyCode::Num2 => Some(if is_shift { b'@' } else { b'2' }),
            KeyCode::Num3 => Some(if is_shift { b'#' } else { b'3' }),
            KeyCode::Num4 => Some(if is_shift { b'$' } else { b'4' }),
            KeyCode::Num5 => Some(if is_shift { b'%' } else { b'5' }),
            KeyCode::Num6 => Some(if is_shift { b'^' } else { b'6' }),
            KeyCode::Num7 => Some(if is_shift { b'&' } else { b'7' }),
            KeyCode::Num8 => Some(if is_shift { b'*' } else { b'8' }),
            KeyCode::Num9 => Some(if is_shift { b'(' } else { b'9' }),
            KeyCode::Num0 => Some(if is_shift { b')' } else { b'0' }),

            KeyCode::Space => Some(b' '),
            KeyCode::Enter | KeyCode::KpEnter => Some(b'\n'),
            KeyCode::Tab => Some(b'\t'),
            KeyCode::Backspace => Some(8),
            KeyCode::Escape => Some(27),

            KeyCode::Minus => Some(if is_shift { b'_' } else { b'-' }),
            KeyCode::Equal => Some(if is_shift { b'+' } else { b'=' }),
            KeyCode::LeftBracket => Some(if is_shift { b'{' } else { b'[' }),
            KeyCode::RightBracket => Some(if is_shift { b'}' } else { b']' }),
            KeyCode::Backslash => Some(if is_shift { b'|' } else { b'\\' }),
            KeyCode::Semicolon => Some(if is_shift { b':' } else { b';' }),
            KeyCode::Apostrophe => Some(if is_shift { b'"' } else { b'\'' }),
            KeyCode::Grave => Some(if is_shift { b'~' } else { b'`' }),
            KeyCode::Comma => Some(if is_shift { b'<' } else { b',' }),
            KeyCode::Period => Some(if is_shift { b'>' } else { b'.' }),
            KeyCode::Slash => Some(if is_shift { b'?' } else { b'/' }),

            _ => None,
        }
    }
}
