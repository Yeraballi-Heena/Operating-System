//! Integrate OS — Multi-Stream Software Audio Mixer
//!
//! Blends multiple concurrent audio playback streams into a master stereo PCM stream
//! with per-track volume, looping control, master attenuation, and saturation clamping.

use alloc::vec::Vec;
use super::buffer::PcmBuffer;
use super::types::{AudioFormat, SampleFormat};

/// A single audio playback channel in the mixer.
#[derive(Debug, Clone)]
pub struct MixerTrack {
    pub id: u32,
    pub buffer: PcmBuffer,
    pub cursor_bytes: usize,
    pub volume: u8,
    pub looping: bool,
    pub active: bool,
}

impl MixerTrack {
    pub fn new(id: u32, buffer: PcmBuffer, volume: u8, looping: bool) -> Self {
        Self {
            id,
            buffer,
            cursor_bytes: 0,
            volume: volume.min(100),
            looping,
            active: true,
        }
    }

    /// Returns remaining bytes in this track's buffer.
    pub fn remaining_bytes(&self) -> usize {
        let total = self.buffer.byte_len();
        if self.cursor_bytes < total {
            total - self.cursor_bytes
        } else {
            0
        }
    }
}

/// Software Audio Mixer.
pub struct AudioMixer {
    pub master_format: AudioFormat,
    tracks: Vec<MixerTrack>,
    next_track_id: u32,
    master_volume: u8,
    muted: bool,
}

impl AudioMixer {
    pub fn new(master_format: AudioFormat) -> Self {
        Self {
            master_format,
            tracks: Vec::new(),
            next_track_id: 1,
            master_volume: 80,
            muted: false,
        }
    }

    /// Adds a PCM buffer to the mixer and begins playback.
    pub fn play_buffer(&mut self, buffer: PcmBuffer, volume: u8, looping: bool) -> u32 {
        let id = self.next_track_id;
        self.next_track_id = self.next_track_id.wrapping_add(1);
        self.tracks.push(MixerTrack::new(id, buffer, volume, looping));
        id
    }

    /// Stops and removes a track by ID.
    pub fn stop_track(&mut self, track_id: u32) {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == track_id) {
            self.tracks.remove(pos);
        }
    }

    /// Stops all currently playing tracks.
    pub fn stop_all(&mut self) {
        self.tracks.clear();
    }

    /// Returns the number of currently active playback tracks.
    pub fn active_tracks_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.active).count()
    }

    /// Sets the master volume (0..100).
    pub fn set_master_volume(&mut self, volume: u8) {
        self.master_volume = volume.min(100);
    }

    /// Returns the current master volume (0..100).
    pub fn master_volume(&self) -> u8 {
        self.master_volume
    }

    /// Sets the mute status.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Returns whether the mixer is currently muted.
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Renders and mixes all active tracks into the provided PCM output slice.
    /// Returns the number of bytes rendered.
    pub fn render_mix(&mut self, output: &mut [u8]) -> usize {
        if output.is_empty() || self.master_format.sample_format != SampleFormat::S16Le {
            return 0;
        }

        // Zero out the output buffer (silence)
        output.fill(0);

        if self.muted || self.master_volume == 0 || self.tracks.is_empty() {
            return output.len();
        }

        let num_samples = output.len() / 2;
        let master_factor = self.master_volume as i32;

        for track in self.tracks.iter_mut() {
            if !track.active {
                continue;
            }

            let track_bytes = track.buffer.as_bytes();
            let track_factor = track.volume as i32;

            for sample_idx in 0..num_samples {
                if track.cursor_bytes + 1 >= track_bytes.len() {
                    if track.looping && !track_bytes.is_empty() {
                        track.cursor_bytes = 0;
                    } else {
                        track.active = false;
                        break;
                    }
                }

                let b0 = track_bytes[track.cursor_bytes];
                let b1 = track_bytes[track.cursor_bytes + 1];
                let sample_in = i16::from_le_bytes([b0, b1]) as i32;
                track.cursor_bytes += 2;

                // Scale by track volume and master volume
                let scaled_sample = (sample_in * track_factor * master_factor) / 10000;

                // Read current accumulator in output
                let out_offset = sample_idx * 2;
                let cur_out = i16::from_le_bytes([output[out_offset], output[out_offset + 1]]) as i32;

                // Accumulate with saturation clamp
                let accumulated = (cur_out + scaled_sample).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                let out_bytes = accumulated.to_le_bytes();
                output[out_offset] = out_bytes[0];
                output[out_offset + 1] = out_bytes[1];
            }
        }

        // Clean up completed non-looping tracks
        self.tracks.retain(|t| t.active);

        output.len()
    }
}
