//! Integrate OS — PC Speaker Hardware Driver
//!
//! Controls the legacy motherboard PC speaker via 8254 PIT Channel 2 (ports 0x42/0x43)
//! and system control port B (0x61) for tone generation, boot beeps, and hardware alerts.

use crate::arch::x86_64::io::{inb, outb};
use crate::arch::x86_64::pit;

const PIT_CHANNEL2_DATA: u16 = 0x42;
const PIT_COMMAND_PORT: u16 = 0x43;
const PC_SPEAKER_PORT: u16 = 0x61;
const PIT_BASE_FREQ: u32 = 1193182; // 1.193182 MHz

/// PC Speaker Controller.
pub struct PcSpeaker;

impl PcSpeaker {
    /// Starts generating a continuous tone at `freq_hz`.
    pub fn start_tone(freq_hz: u32) {
        if freq_hz == 0 || freq_hz > 20000 {
            Self::stop_tone();
            return;
        }

        let divisor = (PIT_BASE_FREQ / freq_hz).clamp(1, 65535) as u16;

        // SAFETY: Direct x86 port I/O to PIT Channel 2 and PC speaker control gate.
        unsafe {
            // Configure PIT Channel 2: Mode 3 (Square Wave Generator), 16-bit binary
            outb(PIT_COMMAND_PORT, 0xB6);

            // Write divisor low byte then high byte
            outb(PIT_CHANNEL2_DATA, (divisor & 0xFF) as u8);
            outb(PIT_CHANNEL2_DATA, ((divisor >> 8) & 0xFF) as u8);

            // Enable speaker bits (bit 0 = timer 2 gate, bit 1 = speaker data)
            let prev = inb(PC_SPEAKER_PORT);
            if (prev & 0x03) != 0x03 {
                outb(PC_SPEAKER_PORT, prev | 0x03);
            }
        }
    }

    /// Stops tone generation and turns off the PC speaker.
    pub fn stop_tone() {
        // SAFETY: Direct port I/O to disable PC speaker gate.
        unsafe {
            let prev = inb(PC_SPEAKER_PORT);
            outb(PC_SPEAKER_PORT, prev & 0xFC);
        }
    }

    /// Plays a blocking beep for `duration_ms` at `freq_hz`.
    pub fn beep(freq_hz: u32, duration_ms: u32) {
        Self::start_tone(freq_hz);

        // Spin-wait ticks via PIT (approx 100 Hz = 10 ms per tick)
        let ticks_to_wait = (duration_ms as u64 + 9) / 10;
        let start = pit::get_ticks();
        while pit::get_ticks() < start + ticks_to_wait {
            core::hint::spin_loop();
        }

        Self::stop_tone();
    }
}
