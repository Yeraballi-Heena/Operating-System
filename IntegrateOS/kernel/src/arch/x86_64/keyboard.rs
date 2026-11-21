//! PS/2 Keyboard Controller Driver (IRQ 1 / Vector 33)
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Reads raw scancodes from port 0x60, decodes standard US QWERTY Scancode Set 1,
//! maintains Shift/Caps state, and queues ASCII characters into an interrupt-safe buffer.
//! Also routes auxiliary bytes (status bit 5 = 0x20) directly to the PS/2 mouse driver
//! so the 8042 output buffer never stalls or misroutes mouse packets.

use crate::serial::inb;
use crate::sync::IrqSpinlock;

pub const KEYBOARD_DATA_PORT: u16 = 0x60;
pub const KEYBOARD_STATUS_PORT: u16 = 0x64;

const BUFFER_CAPACITY: usize = 128;

pub struct KeyboardBuffer {
    buffer: [u8; BUFFER_CAPACITY],
    read_pos: usize,
    write_pos: usize,
    count: usize,
    shift_pressed: bool,
    caps_lock: bool,
}

impl KeyboardBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: [0; BUFFER_CAPACITY],
            read_pos: 0,
            write_pos: 0,
            count: 0,
            shift_pressed: false,
            caps_lock: false,
        }
    }

    pub fn push(&mut self, byte: u8) {
        if self.count < BUFFER_CAPACITY {
            self.buffer[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % BUFFER_CAPACITY;
            self.count += 1;
        }
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            None
        } else {
            let byte = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % BUFFER_CAPACITY;
            self.count -= 1;
            Some(byte)
        }
    }
}

pub static KEYBOARD_QUEUE: IrqSpinlock<KeyboardBuffer> = IrqSpinlock::new(KeyboardBuffer::new());

// Scancode Set 1 US QWERTY translation table (scancodes 0x00..=0x39)
const SCANCODE_TABLE_LOWER: [u8; 58] = [
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, // 0x00..0x0E (8 = Backspace)
    b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', // 0x0F..0x1C
    0, b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', // 0x1D..0x29
    0, b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 0, // 0x2A..0x36
    b'*', 0, b' ', // 0x37..0x39
];

const SCANCODE_TABLE_UPPER: [u8; 58] = [
    0, 27, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 8,
    b'\t', b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', b'\n',
    0, b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':', b'"', b'~',
    0, b'|', b'Z', b'X', b'C', b'V', b'B', b'N', b'M', b'<', b'>', b'?', 0,
    b'*', 0, b' ',
];

/// Processes a single verified keyboard scancode byte (when status bit 5 == 0).
pub fn process_raw_scancode(scancode: u8) -> Option<u8> {
    // Ignore PS/2 ACK (0xFA) or Echo (0xEE) control responses
    if scancode == 0xFA || scancode == 0xFE {
        return None;
    }

    crate::input::process_keyboard_byte(scancode);

    let mut queue = KEYBOARD_QUEUE.lock();

    // Check for key release (break code: bit 7 set)
    if (scancode & 0x80) != 0 {
        let make_code = scancode & 0x7F;
        if make_code == 0x2A || make_code == 0x36 {
            queue.shift_pressed = false;
        }
        return None;
    }

    // Key press (make code)
    match scancode {
        0x2A | 0x36 => {
            queue.shift_pressed = true;
            None
        }
        0x3A => {
            queue.caps_lock = !queue.caps_lock;
            None
        }
        code if (code as usize) < SCANCODE_TABLE_LOWER.len() => {
            let use_upper = queue.shift_pressed ^ queue.caps_lock;
            let ascii = if use_upper {
                SCANCODE_TABLE_UPPER[code as usize]
            } else {
                SCANCODE_TABLE_LOWER[code as usize]
            };

            if ascii != 0 {
                queue.push(ascii);
                Some(ascii)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Drains all pending bytes from the 8042 PS/2 controller (`0x64` / `0x60`),
/// routing keyboard bytes (AUXB=0) to the keyboard decoder and mouse bytes (AUXB=1)
/// to the PS/2 mouse packet state machine.
pub fn drain_8042_controller() -> Option<u8> {
    let mut last_ascii = None;
    for _ in 0..16 {
        // SAFETY: Reading 8042 status port 0x64 and data port 0x60.
        let (has_data, is_mouse, byte) = unsafe {
            let status = inb(KEYBOARD_STATUS_PORT);
            if (status & 0x01) == 0 {
                (false, false, 0)
            } else {
                let data = inb(KEYBOARD_DATA_PORT);
                (true, (status & 0x20) != 0, data)
            }
        };

        if !has_data {
            break;
        }

        if is_mouse {
            let ticks = crate::arch::x86_64::pit::get_ticks();
            crate::input::mouse::MOUSE.lock().process_byte(byte, ticks);
        } else if let Some(ch) = process_raw_scancode(byte) {
            last_ascii = Some(ch);
        }
    }
    last_ascii
}

/// Handles IRQ 1 keyboard interrupt.
pub fn handle_keyboard_interrupt() -> Option<u8> {
    drain_8042_controller()
}
