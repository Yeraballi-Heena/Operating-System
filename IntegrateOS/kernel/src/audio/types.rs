//! Integrate OS — Audio System Types & Core Primitives
//!
//! Defines sample formats, channel configurations, standard sample rates,
//! stream format descriptor, playback state machine, audio errors, and capabilities.

use core::fmt;

/// Supported PCM Audio Sample Formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned integer PCM (0..255, center 128)
    U8,
    /// 16-bit signed little-endian PCM (-32768..32767, center 0)
    S16Le,
    /// 24-bit signed little-endian PCM (in 32-bit container)
    S24Le,
    /// 32-bit signed little-endian PCM
    S32Le,
    /// 32-bit IEEE 754 floating point PCM (-1.0..1.0)
    F32Le,
}

impl SampleFormat {
    /// Returns the number of bytes required per sample.
    #[inline]
    pub const fn bytes_per_sample(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::S16Le => 2,
            Self::S24Le => 4,
            Self::S32Le => 4,
            Self::F32Le => 4,
        }
    }

    /// Returns the bit depth per sample.
    #[inline]
    pub const fn bits_per_sample(self) -> u16 {
        match self {
            Self::U8 => 8,
            Self::S16Le => 16,
            Self::S24Le => 24,
            Self::S32Le => 32,
            Self::F32Le => 32,
        }
    }
}

/// Supported Audio Channel Configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannels {
    /// Single channel (Monaural)
    Mono,
    /// Two channels (Stereo: Left, Right)
    Stereo,
    /// 5.1 Surround (6 channels)
    Surround51,
    /// 7.1 Surround (8 channels)
    Surround71,
}

impl AudioChannels {
    /// Returns the number of discrete audio channels.
    #[inline]
    pub const fn count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround51 => 6,
            Self::Surround71 => 8,
        }
    }
}

/// Standard Sampling Rates (Hz).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRate {
    Rate8000 = 8000,
    Rate11025 = 11025,
    Rate22050 = 22050,
    Rate44100 = 44100,
    Rate48000 = 48000,
    Rate96000 = 96000,
}

impl SampleRate {
    /// Returns sample rate as an integer frequency in Hz.
    #[inline]
    pub const fn hz(self) -> u32 {
        self as u32
    }
}

/// Complete PCM Audio Stream Format Descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_format: SampleFormat,
    pub channels: AudioChannels,
    pub sample_rate: u32,
}

impl AudioFormat {
    /// Standard CD Quality (44.1 kHz, 16-bit Signed Stereo).
    pub const CD_QUALITY: Self = Self {
        sample_format: SampleFormat::S16Le,
        channels: AudioChannels::Stereo,
        sample_rate: 44100,
    };

    /// Standard High-Definition / DVD Quality (48.0 kHz, 16-bit Signed Stereo).
    pub const DVD_QUALITY: Self = Self {
        sample_format: SampleFormat::S16Le,
        channels: AudioChannels::Stereo,
        sample_rate: 48000,
    };

    /// Voice / Low-Bandwidth Mono Quality (16.0 kHz, 16-bit Signed Mono).
    pub const VOICE_MONO: Self = Self {
        sample_format: SampleFormat::S16Le,
        channels: AudioChannels::Mono,
        sample_rate: 16000,
    };

    pub const fn new(sample_format: SampleFormat, channels: AudioChannels, sample_rate: u32) -> Self {
        Self {
            sample_format,
            channels,
            sample_rate,
        }
    }

    /// Size of a single audio frame (one sample per channel) in bytes.
    #[inline]
    pub const fn frame_size(&self) -> usize {
        self.sample_format.bytes_per_sample() * self.channels.count()
    }

    /// Number of bytes transferred per second.
    #[inline]
    pub const fn byte_rate(&self) -> usize {
        self.frame_size() * (self.sample_rate as usize)
    }

    /// Calculates audio duration in milliseconds from byte count.
    #[inline]
    pub fn duration_ms(&self, byte_count: usize) -> u32 {
        let br = self.byte_rate();
        if br == 0 {
            0
        } else {
            ((byte_count as u64 * 1000) / br as u64) as u32
        }
    }

    /// Calculates byte count required for a given duration in milliseconds.
    #[inline]
    pub fn bytes_for_duration(&self, duration_ms: u32) -> usize {
        ((self.byte_rate() as u64 * duration_ms as u64) / 1000) as usize
    }
}

/// Operational state of an audio playback stream or controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioState {
    Stopped,
    Playing,
    Paused,
}

/// Hardware capabilities of an audio controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioCapabilities {
    pub max_channels: u8,
    pub supports_16bit: bool,
    pub supports_24bit: bool,
    pub supports_32bit: bool,
    pub supports_44100: bool,
    pub supports_48000: bool,
    pub supports_96000: bool,
    pub has_hardware_mixer: bool,
    pub dma_buffer_size: usize,
}

impl AudioCapabilities {
    pub const fn virtual_baseline() -> Self {
        Self {
            max_channels: 2,
            supports_16bit: true,
            supports_24bit: true,
            supports_32bit: true,
            supports_44100: true,
            supports_48000: true,
            supports_96000: true,
            has_hardware_mixer: true,
            dma_buffer_size: 65536,
        }
    }
}

/// Audio subsystem error conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    DeviceNotFound,
    DeviceBusy,
    UnsupportedFormat,
    BufferOverflow,
    BufferUnderflow,
    DmaError,
    InvalidParameter,
    HardwareFault,
}

impl AudioError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "Audio device or controller not found",
            Self::DeviceBusy => "Audio device is currently busy",
            Self::UnsupportedFormat => "Requested audio PCM format is unsupported",
            Self::BufferOverflow => "PCM audio buffer overflow",
            Self::BufferUnderflow => "PCM audio buffer underrun",
            Self::DmaError => "Direct Memory Access (DMA) controller error",
            Self::InvalidParameter => "Invalid audio parameter or volume range",
            Self::HardwareFault => "Hardware audio controller fault",
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
