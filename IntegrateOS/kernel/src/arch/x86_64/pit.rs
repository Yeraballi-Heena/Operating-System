//! 8254 Programmable Interval Timer (PIT) Driver
//!
//! Provides the primary periodic hardware timer tick on IRQ 0 (Vector 32).
//! Configured in Square Wave Mode (Mode 3) at a default target frequency of 100 Hz.

use crate::serial::outb;
use core::sync::atomic::{AtomicU64, Ordering};

pub const PIT_CHANNEL0_DATA: u16 = 0x40;
pub const PIT_COMMAND_PORT: u16 = 0x43;

/// Base oscillator frequency of the 8254 PIT (1.193182 MHz).
pub const PIT_BASE_FREQUENCY_HZ: u32 = 1_193_182;

static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// Initializes Channel 0 of the 8254 PIT in Mode 3 (Square Wave Generator)
/// at the specified target frequency (e.g. 100 Hz).
pub fn init(target_freq_hz: u32) {
    let divisor = (PIT_BASE_FREQUENCY_HZ / target_freq_hz).clamp(1, 65535) as u16;

    // SAFETY: Channel 0, access mode low/high byte, mode 3 (square wave), 16-bit binary.
    // Command byte: 00 (Channel 0) | 11 (Access lobyte/hibyte) | 011 (Mode 3) | 0 (Binary) = 0x36.
    unsafe {
        outb(PIT_COMMAND_PORT, 0x36);
        outb(PIT_CHANNEL0_DATA, (divisor & 0xFF) as u8);
        outb(PIT_CHANNEL0_DATA, ((divisor >> 8) & 0xFF) as u8);
    }
}

/// Invoked on every IRQ 0 timer interrupt to increment the system tick counter.
pub fn handle_timer_interrupt() -> u64 {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1
}

/// Returns the total number of hardware timer ticks since boot.
pub fn get_ticks() -> u64 {
    TIMER_TICKS.load(Ordering::Relaxed)
}
