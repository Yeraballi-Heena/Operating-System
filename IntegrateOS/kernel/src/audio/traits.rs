//! Integrate OS — Audio Hardware Driver Traits
//!
//! Provides the uniform hardware abstraction trait for sound cards,
//! DMA controllers, and virtual audio loopback engines.

use super::types::{AudioCapabilities, AudioError, AudioFormat, AudioState};

/// Uniform trait implemented by all audio hardware controllers and virtual audio sinks.
pub trait AudioDriver: Send + Sync {
    /// Human-readable driver name (e.g. "Intel High Definition Audio", "Virtual Audio Sink").
    fn name(&self) -> &str;

    /// Initializes hardware registers, DMA engines, and codecs.
    fn init(&mut self) -> Result<(), AudioError>;

    /// Returns hardware capabilities (sample formats, rates, channel limits, buffer size).
    fn capabilities(&self) -> AudioCapabilities;

    /// Returns a slice of supported PCM stream formats.
    fn supported_formats(&self) -> &[AudioFormat];

    /// Writes PCM audio data to the output DMA ring buffer or FIFO queue.
    /// Returns the number of bytes successfully accepted.
    fn write_pcm(&mut self, data: &[u8]) -> Result<usize, AudioError>;

    /// Sets the master hardware/driver volume (0..100).
    fn set_volume(&mut self, volume: u8) -> Result<(), AudioError>;

    /// Gets the current master volume (0..100).
    fn volume(&self) -> u8;

    /// Pauses current audio playback.
    fn pause(&mut self) -> Result<(), AudioError>;

    /// Resumes playback from paused state.
    fn resume(&mut self) -> Result<(), AudioError>;

    /// Stops playback and resets the stream position.
    fn stop(&mut self) -> Result<(), AudioError>;

    /// Queries the current playback state.
    fn state(&self) -> AudioState;
}
