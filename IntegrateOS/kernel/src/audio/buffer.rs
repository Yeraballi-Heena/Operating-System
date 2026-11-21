//! Integrate OS — PCM Audio Buffer Abstraction
//!
//! Provides dynamically sized PCM sample storage, frame indexing, software volume
//! attenuation, multi-buffer blending, and peak amplitude analysis.

use alloc::vec;
use alloc::vec::Vec;
use super::types::{AudioError, AudioFormat, SampleFormat};

/// Dynamically sized buffer of PCM audio samples.
#[derive(Debug, Clone)]
pub struct PcmBuffer {
    pub format: AudioFormat,
    data: Vec<u8>,
}

impl PcmBuffer {
    /// Creates an empty PCM buffer with a specified format and initial capacity in frames.
    pub fn with_capacity(format: AudioFormat, capacity_frames: usize) -> Self {
        let byte_capacity = capacity_frames * format.frame_size();
        Self {
            format,
            data: Vec::with_capacity(byte_capacity),
        }
    }

    /// Creates a PCM buffer initialized with zeroed silence.
    pub fn silence(format: AudioFormat, frames: usize) -> Self {
        let byte_count = frames * format.frame_size();
        let fill_byte = match format.sample_format {
            SampleFormat::U8 => 128, // 8-bit unsigned center is 128
            _ => 0,                  // Signed formats center is 0
        };
        Self {
            format,
            data: vec![fill_byte; byte_count],
        }
    }

    /// Creates a PCM buffer from an existing raw byte slice.
    pub fn from_bytes(format: AudioFormat, bytes: &[u8]) -> Result<Self, AudioError> {
        let frame_size = format.frame_size();
        if frame_size == 0 || bytes.len() % frame_size != 0 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            format,
            data: bytes.to_vec(),
        })
    }

    /// Number of total bytes currently stored in the buffer.
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    /// Number of audio frames stored in the buffer.
    #[inline]
    pub fn frame_count(&self) -> usize {
        let frame_size = self.format.frame_size();
        if frame_size == 0 {
            0
        } else {
            self.data.len() / frame_size
        }
    }

    /// Duration of audio in milliseconds.
    #[inline]
    pub fn duration_ms(&self) -> u32 {
        self.format.duration_ms(self.data.len())
    }

    /// Immutable reference to the raw underlying PCM byte data.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Mutable reference to the raw underlying PCM byte data.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Appends raw byte data to the end of the buffer.
    pub fn append_bytes(&mut self, bytes: &[u8]) -> Result<(), AudioError> {
        let frame_size = self.format.frame_size();
        if frame_size == 0 || bytes.len() % frame_size != 0 {
            return Err(AudioError::InvalidParameter);
        }
        self.data.extend_from_slice(bytes);
        Ok(())
    }

    /// Appends a 16-bit signed sample (in native S16Le format).
    pub fn push_sample_s16(&mut self, sample: i16) {
        let bytes = sample.to_le_bytes();
        self.data.push(bytes[0]);
        self.data.push(bytes[1]);
    }

    /// Clears all sample data from the buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Reads a 16-bit signed sample at the given sample index.
    pub fn sample_s16(&self, sample_idx: usize) -> Option<i16> {
        let offset = sample_idx * 2;
        if offset + 2 <= self.data.len() {
            let b0 = self.data[offset];
            let b1 = self.data[offset + 1];
            Some(i16::from_le_bytes([b0, b1]))
        } else {
            None
        }
    }

    /// Scales all S16Le samples in place by a volume percentage (0..100).
    pub fn apply_volume(&mut self, volume: u8) {
        if self.format.sample_format != SampleFormat::S16Le {
            return;
        }

        let vol_factor = volume.min(100) as i32;
        for chunk in self.data.chunks_exact_mut(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
            let scaled = (sample * vol_factor) / 100;
            let clamped = scaled.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let bytes = clamped.to_le_bytes();
            chunk[0] = bytes[0];
            chunk[1] = bytes[1];
        }
    }

    /// Calculates the maximum peak amplitude in the buffer (0..32767).
    pub fn peak_amplitude_s16(&self) -> i16 {
        if self.format.sample_format != SampleFormat::S16Le {
            return 0;
        }

        let mut peak: i16 = 0;
        for chunk in self.data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            let abs_val = if sample == i16::MIN { i16::MAX } else { sample.abs() };
            if abs_val > peak {
                peak = abs_val;
            }
        }
        peak
    }

    /// Blends another S16Le buffer into this buffer with saturation clamping to prevent clipping.
    pub fn mix_s16(&mut self, other: &PcmBuffer, gain_percent: u8) -> Result<(), AudioError> {
        if self.format != other.format || self.format.sample_format != SampleFormat::S16Le {
            return Err(AudioError::UnsupportedFormat);
        }

        let min_len = self.data.len().min(other.data.len());
        let gain = gain_percent.min(100) as i32;

        for i in (0..min_len).step_by(2) {
            let s1 = i16::from_le_bytes([self.data[i], self.data[i + 1]]) as i32;
            let s2 = i16::from_le_bytes([other.data[i], other.data[i + 1]]) as i32;

            let mixed = s1 + (s2 * gain) / 100;
            let clamped = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let bytes = clamped.to_le_bytes();

            self.data[i] = bytes[0];
            self.data[i + 1] = bytes[1];
        }

        Ok(())
    }
}
