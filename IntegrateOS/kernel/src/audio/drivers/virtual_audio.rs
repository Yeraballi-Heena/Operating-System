//! Integrate OS — Virtual Audio Driver & Loopback Sink
//!
//! Provides an in-memory loopback audio sink for deterministic testing,
//! sample verification, and audio output on headless / virtualized targets.

use alloc::vec;
use alloc::vec::Vec;
use super::super::traits::AudioDriver;
use super::super::types::{
    AudioCapabilities, AudioError, AudioFormat, AudioState,
};

pub struct VirtualAudioDriver {
    name: &'static str,
    state: AudioState,
    volume: u8,
    ring_buffer: Vec<u8>,
    buffer_capacity: usize,
    total_bytes_written: usize,
    peak_amplitude_seen: i16,
    formats: [AudioFormat; 3],
}

impl VirtualAudioDriver {
    pub const BUFFER_SIZE: usize = 65536; // 64 KiB ring buffer

    pub fn new() -> Self {
        Self {
            name: "Integrate OS Virtual Loopback Audio Sink",
            state: AudioState::Stopped,
            volume: 80,
            ring_buffer: vec![0; Self::BUFFER_SIZE],
            buffer_capacity: Self::BUFFER_SIZE,
            total_bytes_written: 0,
            peak_amplitude_seen: 0,
            formats: [
                AudioFormat::CD_QUALITY,
                AudioFormat::DVD_QUALITY,
                AudioFormat::VOICE_MONO,
            ],
        }
    }

    /// Returns the total cumulative bytes sent to this driver.
    pub fn total_bytes_written(&self) -> usize {
        self.total_bytes_written
    }

    /// Returns the peak sample amplitude observed in written audio.
    pub fn peak_amplitude_seen(&self) -> i16 {
        self.peak_amplitude_seen
    }

    /// Clears the loopback ring buffer and resets counters.
    pub fn reset_buffer(&mut self) {
        self.ring_buffer.fill(0);
        self.total_bytes_written = 0;
        self.peak_amplitude_seen = 0;
    }
}

impl AudioDriver for VirtualAudioDriver {
    fn name(&self) -> &str {
        self.name
    }

    fn init(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Stopped;
        self.reset_buffer();
        Ok(())
    }

    fn capabilities(&self) -> AudioCapabilities {
        AudioCapabilities::virtual_baseline()
    }

    fn supported_formats(&self) -> &[AudioFormat] {
        &self.formats
    }

    fn write_pcm(&mut self, data: &[u8]) -> Result<usize, AudioError> {
        if data.is_empty() {
            return Ok(0);
        }

        let write_len = data.len().min(self.buffer_capacity);

        // Track peak amplitude for 16-bit samples
        for chunk in data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            let abs_val = if sample == i16::MIN { i16::MAX } else { sample.abs() };
            if abs_val > self.peak_amplitude_seen {
                self.peak_amplitude_seen = abs_val;
            }
        }

        // Copy into ring buffer with wrap-around
        let start_pos = self.total_bytes_written % self.buffer_capacity;
        for (i, &byte) in data.iter().take(write_len).enumerate() {
            let idx = (start_pos + i) % self.buffer_capacity;
            self.ring_buffer[idx] = byte;
        }

        self.total_bytes_written += write_len;
        self.state = AudioState::Playing;

        Ok(write_len)
    }

    fn set_volume(&mut self, volume: u8) -> Result<(), AudioError> {
        self.volume = volume.min(100);
        Ok(())
    }

    fn volume(&self) -> u8 {
        self.volume
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Playing;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Stopped;
        Ok(())
    }

    fn state(&self) -> AudioState {
        self.state
    }
}
