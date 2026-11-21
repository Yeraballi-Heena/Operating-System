//! Integrate OS — 16550 UART Serial Driver (`COM1` at `0x3F8`)
//!
//! Provides deterministic early-boot and runtime diagnostic output over COM1.
//! Configured at 115200 baud, 8 data bits, no parity, 1 stop bit (8N1).

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

/// Base I/O port for `COM1`.
pub const COM1_PORT: u16 = 0x3F8;

static SERIAL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Writes a single byte `val` to x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is a valid hardware I/O port and writing `val` does not
/// violate memory safety.
#[inline(always)]
pub unsafe fn outb(port: u16, val: u8) {
    // SAFETY: Direct x86 port I/O instruction in Ring 0.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads a single byte from x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is a valid hardware I/O port.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    // SAFETY: Direct x86 port I/O instruction in Ring 0.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            out("al") val,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

/// Initializes the `COM1` (`0x3F8`) 16550 UART controller.
pub fn init() {
    // SAFETY: Standard 16550 UART initialization sequence on legacy port 0x3F8.
    unsafe {
        outb(COM1_PORT + 1, 0x00); // Disable UART interrupts
        outb(COM1_PORT + 3, 0x80); // Enable DLAB (set baud rate divisor)
        outb(COM1_PORT + 0, 0x01); // Divisor low byte = 1 (115200 baud)
        outb(COM1_PORT + 1, 0x00); // Divisor high byte = 0
        outb(COM1_PORT + 3, 0x03); // 8 bits, no parity, one stop bit (8N1), clear DLAB
        outb(COM1_PORT + 2, 0xC7); // Enable FIFO, clear TX/RX queues, 14-byte threshold
        outb(COM1_PORT + 4, 0x0B); // IRQs enabled, RTS/DSR set
    }
    SERIAL_INITIALIZED.store(true, Ordering::Release);
}

#[inline]
fn is_transmit_empty() -> bool {
    // SAFETY: Reading Line Status Register (COM1 + 5) is side-effect free for TX status.
    unsafe { (inb(COM1_PORT + 5) & 0x20) != 0 }
}

/// Writes a single byte to `COM1`, polling the Line Status Register until ready.
pub fn write_byte(byte: u8) {
    if !SERIAL_INITIALIZED.load(Ordering::Acquire) {
        init();
    }
    if byte == b'\n' {
        write_raw_byte(b'\r');
    }
    write_raw_byte(byte);
}

#[inline]
fn write_raw_byte(byte: u8) {
    let mut spins = 0u32;
    while !is_transmit_empty() && spins < 100_000 {
        core::hint::spin_loop();
        spins += 1;
    }
    // SAFETY: Writing to COM1 Transmit Holding Register (0x3F8).
    unsafe {
        outb(COM1_PORT, byte);
    }
}

/// Writes a UTF-8 string slice to `COM1`.
pub fn write_str(s: &str) {
    for b in s.bytes() {
        write_byte(b);
    }
}

/// Zero-sized formatting adapter for `COM1`.
pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
