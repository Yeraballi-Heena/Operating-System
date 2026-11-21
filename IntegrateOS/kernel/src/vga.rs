//! Integrate OS — VGA Text Mode Console (`80x25` at physical `0xB8000`)
//!
//! Renders kernel boot banners, diagnostic logs, and panic screens directly to the
//! standard VGA text buffer so VirtualBox displays live output on screen.

use crate::serial::outb;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

pub const VGA_BUFFER_ADDR: usize = 0xB8000;
pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[inline(always)]
pub const fn color_attr(fg: Color, bg: Color) -> u8 {
    ((bg as u8) << 4) | (fg as u8)
}

static CURSOR_ROW: AtomicUsize = AtomicUsize::new(0);
static CURSOR_COL: AtomicUsize = AtomicUsize::new(0);
static CURRENT_ATTR: AtomicU8 = AtomicU8::new(color_attr(Color::LightGray, Color::Black));

#[inline(always)]
fn vga_cell_ptr(row: usize, col: usize) -> *mut u16 {
    let index = row * VGA_WIDTH + col;
    (VGA_BUFFER_ADDR as *mut u16).wrapping_add(index)
}

/// Clears the 80x25 VGA screen and resets the cursor to `(0, 0)`.
pub fn init() {
    let attr = color_attr(Color::LightGray, Color::Black);
    CURRENT_ATTR.store(attr, Ordering::Relaxed);
    let blank: u16 = ((attr as u16) << 8) | (b' ' as u16);

    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            // SAFETY: 0xB8000 is identity-mapped and valid for 80x25 u16 cells.
            unsafe {
                core::ptr::write_volatile(vga_cell_ptr(row, col), blank);
            }
        }
    }
    CURSOR_ROW.store(0, Ordering::Relaxed);
    CURSOR_COL.store(0, Ordering::Relaxed);
    update_hw_cursor(0, 0);
}

/// Sets the current foreground and background color attribute for subsequent writes.
pub fn set_color(fg: Color, bg: Color) {
    CURRENT_ATTR.store(color_attr(fg, bg), Ordering::Relaxed);
}

fn scroll_up() {
    let attr = CURRENT_ATTR.load(Ordering::Relaxed);
    let blank: u16 = ((attr as u16) << 8) | (b' ' as u16);

    for row in 1..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            // SAFETY: Reading and writing within bounds of 80x25 VGA buffer at 0xB8000.
            unsafe {
                let ch = core::ptr::read_volatile(vga_cell_ptr(row, col));
                core::ptr::write_volatile(vga_cell_ptr(row - 1, col), ch);
            }
        }
    }
    for col in 0..VGA_WIDTH {
        // SAFETY: Clearing the bottom row of 80x25 VGA buffer.
        unsafe {
            core::ptr::write_volatile(vga_cell_ptr(VGA_HEIGHT - 1, col), blank);
        }
    }
}

fn update_hw_cursor(row: usize, col: usize) {
    let pos: u16 = (row * VGA_WIDTH + col) as u16;
    // SAFETY: Standard VGA CRT controller cursor position registers (0x3D4 / 0x3D5).
    unsafe {
        outb(0x3D4, 0x0F);
        outb(0x3D5, (pos & 0xFF) as u8);
        outb(0x3D4, 0x0E);
        outb(0x3D5, ((pos >> 8) & 0xFF) as u8);
    }
}

/// Writes a single ASCII byte to the VGA text buffer.
pub fn write_byte(byte: u8) {
    let mut row = CURSOR_ROW.load(Ordering::Relaxed);
    let mut col = CURSOR_COL.load(Ordering::Relaxed);
    let attr = CURRENT_ATTR.load(Ordering::Relaxed);

    match byte {
        b'\n' => {
            col = 0;
            row += 1;
        }
        b'\r' => {
            col = 0;
        }
        b => {
            let printable = if (0x20..=0x7E).contains(&b) { b } else { b'?' };
            let cell: u16 = ((attr as u16) << 8) | (printable as u16);
            // SAFETY: row < VGA_HEIGHT and col < VGA_WIDTH are maintained by this function.
            unsafe {
                core::ptr::write_volatile(vga_cell_ptr(row, col), cell);
            }
            col += 1;
            if col >= VGA_WIDTH {
                col = 0;
                row += 1;
            }
        }
    }

    if row >= VGA_HEIGHT {
        scroll_up();
        row = VGA_HEIGHT - 1;
    }

    CURSOR_ROW.store(row, Ordering::Relaxed);
    CURSOR_COL.store(col, Ordering::Relaxed);
    update_hw_cursor(row, col);
}

/// Writes a string slice to the VGA text buffer.
pub fn write_str(s: &str) {
    for b in s.bytes() {
        write_byte(b);
    }
}

pub struct VgaWriter;

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
