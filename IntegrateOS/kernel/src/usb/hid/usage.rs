//! Integrate OS — USB HID Usage Tables (Page 0x07 Keyboard / Keypad)
//!
//! Maps standard USB HID scancodes to OS KeyCodes and ASCII characters.

use crate::input::events::KeyCode;

/// Translates a USB HID Usage ID (Page 0x07) into the OS `KeyCode` abstraction.
pub fn hid_usage_to_keycode(usage: u8) -> KeyCode {
    match usage {
        0x04 => KeyCode::A,
        0x05 => KeyCode::B,
        0x06 => KeyCode::C,
        0x07 => KeyCode::D,
        0x08 => KeyCode::E,
        0x09 => KeyCode::F,
        0x0A => KeyCode::G,
        0x0B => KeyCode::H,
        0x0C => KeyCode::I,
        0x0D => KeyCode::J,
        0x0E => KeyCode::K,
        0x0F => KeyCode::L,
        0x10 => KeyCode::M,
        0x11 => KeyCode::N,
        0x12 => KeyCode::O,
        0x13 => KeyCode::P,
        0x14 => KeyCode::Q,
        0x15 => KeyCode::R,
        0x16 => KeyCode::S,
        0x17 => KeyCode::T,
        0x18 => KeyCode::U,
        0x19 => KeyCode::V,
        0x1A => KeyCode::W,
        0x1B => KeyCode::X,
        0x1C => KeyCode::Y,
        0x1D => KeyCode::Z,

        0x1E => KeyCode::Num1,
        0x1F => KeyCode::Num2,
        0x20 => KeyCode::Num3,
        0x21 => KeyCode::Num4,
        0x22 => KeyCode::Num5,
        0x23 => KeyCode::Num6,
        0x24 => KeyCode::Num7,
        0x25 => KeyCode::Num8,
        0x26 => KeyCode::Num9,
        0x27 => KeyCode::Num0,

        0x28 => KeyCode::Enter,
        0x29 => KeyCode::Escape,
        0x2A => KeyCode::Backspace,
        0x2B => KeyCode::Tab,
        0x2C => KeyCode::Space,
        0x2D => KeyCode::Minus,
        0x2E => KeyCode::Equal,
        0x2F => KeyCode::LeftBracket,
        0x30 => KeyCode::RightBracket,
        0x31 => KeyCode::Backslash,
        0x33 => KeyCode::Semicolon,
        0x34 => KeyCode::Apostrophe,
        0x35 => KeyCode::Grave,
        0x36 => KeyCode::Comma,
        0x37 => KeyCode::Period,
        0x38 => KeyCode::Slash,
        0x39 => KeyCode::CapsLock,

        0x3A => KeyCode::F1,
        0x3B => KeyCode::F2,
        0x3C => KeyCode::F3,
        0x3D => KeyCode::F4,
        0x3E => KeyCode::F5,
        0x3F => KeyCode::F6,
        0x40 => KeyCode::F7,
        0x41 => KeyCode::F8,
        0x42 => KeyCode::F9,
        0x43 => KeyCode::F10,
        0x44 => KeyCode::F11,
        0x45 => KeyCode::F12,

        0x4F => KeyCode::Right,
        0x50 => KeyCode::Left,
        0x51 => KeyCode::Down,
        0x52 => KeyCode::Up,

        0xE0 => KeyCode::LeftCtrl,
        0xE1 => KeyCode::LeftShift,
        0xE2 => KeyCode::LeftAlt,
        0xE3 => KeyCode::LeftSuper,
        0xE4 => KeyCode::RightCtrl,
        0xE5 => KeyCode::RightShift,
        0xE6 => KeyCode::RightAlt,
        0xE7 => KeyCode::RightSuper,

        _ => KeyCode::Unknown,
    }
}

/// Translates a USB HID Usage ID to an ASCII character, respecting Shift modifier.
pub fn hid_usage_to_ascii(usage: u8, shift: bool) -> Option<u8> {
    match usage {
        0x04..=0x1D => {
            let base = if shift { b'A' } else { b'a' };
            Some(base + (usage - 0x04))
        }
        0x1E => Some(if shift { b'!' } else { b'1' }),
        0x1F => Some(if shift { b'@' } else { b'2' }),
        0x20 => Some(if shift { b'#' } else { b'3' }),
        0x21 => Some(if shift { b'$' } else { b'4' }),
        0x22 => Some(if shift { b'%' } else { b'5' }),
        0x23 => Some(if shift { b'^' } else { b'6' }),
        0x24 => Some(if shift { b'&' } else { b'7' }),
        0x25 => Some(if shift { b'*' } else { b'8' }),
        0x26 => Some(if shift { b'(' } else { b'9' }),
        0x27 => Some(if shift { b')' } else { b'0' }),

        0x28 => Some(b'\n'), // Enter
        0x29 => Some(0x1B), // Escape
        0x2A => Some(0x08), // Backspace
        0x2B => Some(b'\t'), // Tab
        0x2C => Some(b' '),  // Space
        0x2D => Some(if shift { b'_' } else { b'-' }),
        0x2E => Some(if shift { b'+' } else { b'=' }),
        0x2F => Some(if shift { b'{' } else { b'[' }),
        0x30 => Some(if shift { b'}' } else { b']' }),
        0x31 => Some(if shift { b'|' } else { b'\\' }),
        0x33 => Some(if shift { b':' } else { b';' }),
        0x34 => Some(if shift { b'"' } else { b'\'' }),
        0x35 => Some(if shift { b'~' } else { b'`' }),
        0x36 => Some(if shift { b'<' } else { b',' }),
        0x37 => Some(if shift { b'>' } else { b'.' }),
        0x38 => Some(if shift { b'?' } else { b'/' }),

        _ => None,
    }
}
