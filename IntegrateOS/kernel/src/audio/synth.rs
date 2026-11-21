//! Integrate OS — Audio Waveform Synthesizer & Tone Generator
//!
//! Generates mathematical waveforms (Sine, Square, Sawtooth, Triangle, Noise)
//! and system sound presets in standard 16-bit PCM format without external math libraries.

use super::buffer::PcmBuffer;
use super::types::{AudioChannels, AudioFormat};

/// Supported mathematical audio waveforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    /// Smooth fundamental sinusoidal wave (harmonic purity)
    Sine,
    /// Square pulse wave (50% duty cycle, rich odd harmonics)
    Square,
    /// Sawtooth linear ramp wave (full harmonic spectrum)
    Sawtooth,
    /// Triangle linear symmetrical wave (soft flute-like tone)
    Triangle,
    /// Pseudo-random white noise (percussion, hiss, sound effects)
    Noise,
}

/// Standard musical note frequencies in Hertz (Equal Temperament, A4 = 440 Hz).
pub struct Notes;

impl Notes {
    pub const C3: f32 = 130.81;
    pub const D3: f32 = 146.83;
    pub const E3: f32 = 164.81;
    pub const F3: f32 = 174.61;
    pub const G3: f32 = 196.00;
    pub const A3: f32 = 220.00;
    pub const B3: f32 = 246.94;

    pub const C4: f32 = 261.63; // Middle C
    pub const D4: f32 = 293.66;
    pub const E4: f32 = 329.63;
    pub const F4: f32 = 349.23;
    pub const G4: f32 = 392.00;
    pub const A4: f32 = 440.00; // Concert Pitch
    pub const B4: f32 = 493.88;

    pub const C5: f32 = 523.25;
    pub const D5: f32 = 587.33;
    pub const E5: f32 = 659.25;
    pub const G5: f32 = 783.99;
    pub const A5: f32 = 880.00;
}

/// System audio sound presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPreset {
    /// Ascending major chord (C4 -> E4 -> G4)
    StartupChime,
    /// Short 880 Hz sine alert beep
    AlertBeep,
    /// Friendly two-tone notification (D5 -> A5)
    NotifyTone,
    /// Low harsh 150 Hz square error buzz
    ErrorBuzz,
}

/// Sound Synthesizer engine.
pub struct AudioSynth;

impl AudioSynth {
    /// High-precision, no_std Bhaskara I polynomial sine approximation.
    /// Input: normalized phase `p` in `[0.0, 1.0)`.
    /// Output: sine value in `[-1.0, 1.0]`.
    pub fn approx_sin(phase: f32) -> f32 {
        let p = phase - (phase as u32 as f32); // fractional part in [0, 1)
        const PI: f32 = 3.14159265359;

        if p < 0.5 {
            let th = p * 2.0 * PI;
            (16.0 * th * (PI - th)) / (5.0 * PI * PI - 4.0 * th * (PI - th))
        } else {
            let th = (p - 0.5) * 2.0 * PI;
            -((16.0 * th * (PI - th)) / (5.0 * PI * PI - 4.0 * th * (PI - th)))
        }
    }

    /// Evaluates a waveform at a given normalized phase `[0.0, 1.0)` with a noise seed.
    pub fn sample_waveform(waveform: Waveform, phase: f32, noise_state: &mut u32) -> f32 {
        let p = phase - (phase as u32 as f32);
        match waveform {
            Waveform::Sine => Self::approx_sin(p),
            Waveform::Square => {
                if p < 0.5 { 1.0 } else { -1.0 }
            }
            Waveform::Sawtooth => {
                2.0 * p - 1.0
            }
            Waveform::Triangle => {
                if p < 0.5 {
                    4.0 * p - 1.0
                } else {
                    3.0 - 4.0 * p
                }
            }
            Waveform::Noise => {
                // Linear Congruential Generator (LCG)
                *noise_state = noise_state.wrapping_mul(1664525).wrapping_add(1013904223);
                let normalized = (*noise_state >> 16) as f32 / 32767.5 - 1.0;
                normalized.clamp(-1.0, 1.0)
            }
        }
    }

    /// Generates a synthesized PCM buffer for a given tone.
    pub fn generate_tone(
        waveform: Waveform,
        freq_hz: f32,
        duration_ms: u32,
        volume: u8,
        format: AudioFormat,
    ) -> PcmBuffer {
        let total_frames = ((format.sample_rate as u64 * duration_ms as u64) / 1000) as usize;
        let mut buffer = PcmBuffer::with_capacity(format, total_frames);

        let phase_increment = if format.sample_rate > 0 {
            freq_hz / (format.sample_rate as f32)
        } else {
            0.0
        };

        let mut phase: f32 = 0.0;
        let mut noise_state: u32 = 0x12345678;
        let amp_scale = (volume.min(100) as f32 / 100.0) * 32000.0;

        for _ in 0..total_frames {
            let sample_f32 = Self::sample_waveform(waveform, phase, &mut noise_state);
            let sample_i16 = (sample_f32 * amp_scale).clamp(-32767.0, 32767.0) as i16;

            match format.channels {
                AudioChannels::Mono => {
                    buffer.push_sample_s16(sample_i16);
                }
                AudioChannels::Stereo => {
                    buffer.push_sample_s16(sample_i16);
                    buffer.push_sample_s16(sample_i16);
                }
                _ => {
                    for _ in 0..format.channels.count() {
                        buffer.push_sample_s16(sample_i16);
                    }
                }
            }

            phase += phase_increment;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }

        buffer
    }

    /// Generates a pre-configured system sound preset.
    pub fn generate_preset(preset: SoundPreset, volume: u8, format: AudioFormat) -> PcmBuffer {
        match preset {
            SoundPreset::StartupChime => {
                let note_dur = 80;
                let mut buf = Self::generate_tone(Waveform::Sine, Notes::C4, note_dur, volume, format);
                let e4 = Self::generate_tone(Waveform::Sine, Notes::E4, note_dur, volume, format);
                let g4 = Self::generate_tone(Waveform::Sine, Notes::G4, note_dur * 2, volume, format);
                let _ = buf.append_bytes(e4.as_bytes());
                let _ = buf.append_bytes(g4.as_bytes());
                buf
            }
            SoundPreset::AlertBeep => {
                Self::generate_tone(Waveform::Sine, Notes::A5, 60, volume, format)
            }
            SoundPreset::NotifyTone => {
                let mut buf = Self::generate_tone(Waveform::Triangle, Notes::D5, 70, volume, format);
                let a5 = Self::generate_tone(Waveform::Triangle, Notes::A5, 120, volume, format);
                let _ = buf.append_bytes(a5.as_bytes());
                buf
            }
            SoundPreset::ErrorBuzz => {
                Self::generate_tone(Waveform::Square, 150.0, 180, volume, format)
            }
        }
    }
}
