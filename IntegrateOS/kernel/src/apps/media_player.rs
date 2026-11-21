//! Integrate OS — Multimedia Player Application (`MediaPlayerApp`)
//!
//! Provides video playback (IVID RLE-encoded video streams) and chiptune/audio playback
//! using the bare-metal kernel graphics canvas and hardware PC speaker.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use crate::audio::speaker::PcSpeaker;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};

pub static CLIP_D1_DATA: &[u8] = include_bytes!("../assets/videos/clip_d1.vid");
pub static CLIP_D2_DATA: &[u8] = include_bytes!("../assets/videos/clip_d2.vid");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Video,
    Audio,
}

#[derive(Clone)]
pub struct MediaTrack {
    pub title: &'static str,
    pub media_type: MediaType,
    pub data: Option<&'static [u8]>,
    pub duration_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Decoded video frame metadata
struct VideoHeader {
    width: usize,
    height: usize,
    fps: u32,
    total_frames: usize,
}

pub struct MediaPlayerApp {
    pub tracks: Vec<MediaTrack>,
    pub current_track_idx: usize,
    pub state: PlaybackState,
    pub current_frame: usize,
    pub total_frames: usize,
    pub video_width: usize,
    pub video_height: usize,
    pub frame_timer_ms: u32,
    pub frame_interval_ms: u32,
    pub frame_offsets: Vec<(usize, usize)>, // (offset, len) in data slice
    pub pixel_buffer: Vec<u16>,             // RGB565 frame pixels
    // Audio synthesis playback
    pub current_note_idx: usize,
    pub note_timer_ms: u32,
    pub spectrum_heights: [u32; 16],
    pub spectrum_timer_ms: u32,
    // Interactive button hover/press states
    pub hovered_button: Option<usize>, // 0=Prev, 1=Play/Pause, 2=Stop, 3=Next
    pub pressed_button: Option<usize>,
    pub status_banner: String,
}

// Melody note frequencies: (freq_hz, duration_ms)
static SYNTHWAVE_MELODY: &[(u32, u32)] = &[
    (261, 250), (293, 250), (329, 250), (392, 350),
    (329, 250), (392, 250), (440, 350), (523, 500),
    (440, 250), (392, 250), (329, 250), (261, 400),
    (220, 250), (261, 250), (329, 250), (392, 500),
    (440, 200), (523, 200), (587, 200), (659, 400),
    (587, 200), (523, 200), (440, 200), (392, 400),
    (329, 250), (293, 250), (261, 500), (196, 600),
];

static ARCADE_MELODY: &[(u32, u32)] = &[
    (440, 150), (493, 150), (523, 150), (587, 200),
    (659, 300), (587, 150), (659, 150), (783, 400),
    (659, 150), (587, 150), (523, 200), (440, 300),
    (392, 150), (440, 150), (523, 300), (659, 500),
];

impl MediaPlayerApp {
    pub fn new() -> Self {
        let tracks = vec![
            MediaTrack {
                title: "D:\\ WhatsApp Video 1 (12.49.48)",
                media_type: MediaType::Video,
                data: Some(CLIP_D1_DATA),
                duration_seconds: 12,
            },
            MediaTrack {
                title: "D:\\ WhatsApp Video 2 (12.53.37)",
                media_type: MediaType::Video,
                data: Some(CLIP_D2_DATA),
                duration_seconds: 12,
            },
            MediaTrack {
                title: "Cyberpunk Synthwave (PC Speaker)",
                media_type: MediaType::Audio,
                data: None,
                duration_seconds: 10,
            },
            MediaTrack {
                title: "8-Bit Arcade Hero Melody",
                media_type: MediaType::Audio,
                data: None,
                duration_seconds: 8,
            },
        ];

        let mut app = Self {
            tracks,
            current_track_idx: 0,
            state: PlaybackState::Playing,
            current_frame: 0,
            total_frames: 0,
            video_width: 120,
            video_height: 68,
            frame_timer_ms: 0,
            frame_interval_ms: 166, // 6 fps
            frame_offsets: Vec::new(),
            pixel_buffer: vec![0u16; 120 * 68],
            current_note_idx: 0,
            note_timer_ms: 0,
            spectrum_heights: [4, 8, 14, 22, 30, 26, 18, 12, 16, 24, 28, 20, 14, 10, 6, 2],
            spectrum_timer_ms: 0,
            hovered_button: None,
            pressed_button: None,
            status_banner: String::from("Media Player Ready | Playing D:\\ Video 1"),
        };

        app.load_track(0);
        app.play();
        app
    }

    fn parse_video_header(data: &[u8]) -> Option<(VideoHeader, Vec<(usize, usize)>)> {
        if data.len() < 16 || &data[0..4] != b"IVID" {
            return None;
        }

        let width = u16::from_le_bytes([data[4], data[5]]) as usize;
        let height = u16::from_le_bytes([data[6], data[7]]) as usize;
        let fps = data[8] as u32;
        let total_frames = u16::from_le_bytes([data[9], data[10]]) as usize;

        let mut offsets = Vec::new();
        let mut idx = 16;
        for _ in 0..total_frames {
            if idx + 4 > data.len() {
                break;
            }
            let frame_len = u32::from_le_bytes([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]) as usize;
            idx += 4;
            if idx + frame_len > data.len() {
                break;
            }
            offsets.push((idx, frame_len));
            idx += frame_len;
        }

        Some((
            VideoHeader {
                width,
                height,
                fps: fps.max(1),
                total_frames: offsets.len(),
            },
            offsets,
        ))
    }

    pub fn load_track(&mut self, idx: usize) {
        if idx >= self.tracks.len() {
            return;
        }
        self.stop();
        self.current_track_idx = idx;
        let track_title = self.tracks[idx].title;
        let media_type = self.tracks[idx].media_type;
        let track_data = self.tracks[idx].data;

        if media_type == MediaType::Video {
            if let Some(data) = track_data {
                if let Some((header, offsets)) = Self::parse_video_header(data) {
                    self.video_width = header.width;
                    self.video_height = header.height;
                    self.total_frames = header.total_frames;
                    self.frame_interval_ms = 1000 / header.fps;
                    self.frame_offsets = offsets;
                    self.pixel_buffer = vec![0u16; header.width * header.height];
                    self.decode_current_frame();
                }
            }
        } else {
            self.current_note_idx = 0;
            self.note_timer_ms = 0;
        }

        self.status_banner = format!("Loaded: {}", track_title);
    }

    fn decode_current_frame(&mut self) {
        let track = &self.tracks[self.current_track_idx];
        if track.media_type != MediaType::Video {
            return;
        }
        let Some(data) = track.data else { return };
        if self.current_frame >= self.frame_offsets.len() {
            return;
        }

        let (off, len) = self.frame_offsets[self.current_frame];
        let frame_slice = &data[off..off + len];

        let mut out_idx = 0;
        let max_pixels = self.pixel_buffer.len();
        let mut in_idx = 0;

        while in_idx + 3 <= frame_slice.len() && out_idx < max_pixels {
            let count = frame_slice[in_idx] as usize;
            let val = u16::from_le_bytes([frame_slice[in_idx + 1], frame_slice[in_idx + 2]]);
            in_idx += 3;

            for _ in 0..count {
                if out_idx < max_pixels {
                    self.pixel_buffer[out_idx] = val;
                    out_idx += 1;
                } else {
                    break;
                }
            }
        }
    }

    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
        self.status_banner = format!("▶ Playing: {}", self.tracks[self.current_track_idx].title);
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
        PcSpeaker::stop_tone();
        self.status_banner = String::from("❚❚ Paused");
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_frame = 0;
        self.current_note_idx = 0;
        self.frame_timer_ms = 0;
        self.note_timer_ms = 0;
        PcSpeaker::stop_tone();
        self.decode_current_frame();
        self.status_banner = String::from("■ Stopped");
    }

    pub fn next_track(&mut self) {
        let next = (self.current_track_idx + 1) % self.tracks.len();
        self.load_track(next);
        self.play();
    }

    pub fn prev_track(&mut self) {
        let prev = if self.current_track_idx == 0 {
            self.tracks.len().saturating_sub(1)
        } else {
            self.current_track_idx - 1
        };
        self.load_track(prev);
        self.play();
    }
}

impl Application for MediaPlayerApp {
    fn name(&self) -> &str {
        "Media Player"
    }

    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError> {
        env.set_title("Media Player - HD Video & Sound");
        env.request_redraw();
        Ok(())
    }

    fn on_update(&mut self, env: &mut AppEnv, delta_ms: u32) {
        if self.state != PlaybackState::Playing {
            return;
        }

        let is_video = self.tracks[self.current_track_idx].media_type == MediaType::Video;

        if is_video {
            self.frame_timer_ms += delta_ms;
            if self.frame_timer_ms >= self.frame_interval_ms {
                self.frame_timer_ms = 0;
                if self.total_frames > 0 {
                    self.current_frame = (self.current_frame + 1) % self.total_frames;
                    self.decode_current_frame();
                    env.request_redraw();
                }
            }
        } else {
            // Audio track playback
            let melody = if self.current_track_idx == 2 {
                SYNTHWAVE_MELODY
            } else {
                ARCADE_MELODY
            };

            self.note_timer_ms += delta_ms;
            self.spectrum_timer_ms += delta_ms;

            if self.spectrum_timer_ms >= 80 {
                self.spectrum_timer_ms = 0;
                // Animate spectrum analyzer bars
                for (i, h) in self.spectrum_heights.iter_mut().enumerate() {
                    let seed = ((self.note_timer_ms + (i as u32 * 17)) % 40) + 4;
                    *h = seed;
                }
                env.request_redraw();
            }

            if self.current_note_idx < melody.len() {
                let (freq, dur) = melody[self.current_note_idx];
                PcSpeaker::start_tone(freq);
                if self.note_timer_ms >= dur {
                    self.note_timer_ms = 0;
                    self.current_note_idx += 1;
                    if self.current_note_idx >= melody.len() {
                        self.current_note_idx = 0; // Loop melody
                    }
                }
            }
        }
    }

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        match *event {
            AppEvent::Input(WidgetEvent::MouseMove { x, y }) => {
                // Check button hover (controls at y in 250..280)
                let prev_hover = self.hovered_button;
                if (250..=282).contains(&y) {
                    if (20..74).contains(&x) {
                        self.hovered_button = Some(0); // Prev
                    } else if (80..146).contains(&x) {
                        self.hovered_button = Some(1); // Play/Pause
                    } else if (152..206).contains(&x) {
                        self.hovered_button = Some(2); // Stop
                    } else if (212..266).contains(&x) {
                        self.hovered_button = Some(3); // Next
                    } else {
                        self.hovered_button = None;
                    }
                } else {
                    self.hovered_button = None;
                }

                if prev_hover != self.hovered_button {
                    env.request_redraw();
                }
            }
            AppEvent::Input(WidgetEvent::MouseDown { x, y, button: MouseButton::Left }) => {
                self.pressed_button = self.hovered_button;

                // Transport buttons
                if (250..=282).contains(&y) {
                    if (20..74).contains(&x) {
                        self.prev_track();
                        env.request_redraw();
                        return;
                    } else if (80..146).contains(&x) {
                        if self.state == PlaybackState::Playing {
                            self.pause();
                        } else {
                            self.play();
                        }
                        env.request_redraw();
                        return;
                    } else if (152..206).contains(&x) {
                        self.stop();
                        env.request_redraw();
                        return;
                    } else if (212..266).contains(&x) {
                        self.next_track();
                        env.request_redraw();
                        return;
                    }
                }

                // Scrubber seek bar click (y in 234..246, x in 20..380)
                if (234..=246).contains(&y) && (20..=380).contains(&x) {
                    let progress = ((x - 20) as f32) / 360.0;
                    if self.total_frames > 0 {
                        self.current_frame = ((self.total_frames as f32) * progress) as usize;
                        if self.current_frame >= self.total_frames {
                            self.current_frame = self.total_frames - 1;
                        }
                        self.decode_current_frame();
                    }
                    env.request_redraw();
                    return;
                }

                // Playlist track selection click (right panel: x in 384..560, y in 66..340)
                if x >= 384 && (66..=340).contains(&y) {
                    let row = ((y - 66) / 38) as usize;
                    if row < self.tracks.len() {
                        self.load_track(row);
                        self.play();
                        env.request_redraw();
                    }
                }
            }
            AppEvent::Input(WidgetEvent::MouseUp { .. }) => {
                self.pressed_button = None;
                env.request_redraw();
            }
            AppEvent::Input(WidgetEvent::KeyDown { code, .. }) => match code {
                KeyCode::Space => {
                    if self.state == PlaybackState::Playing {
                        self.pause();
                    } else {
                        self.play();
                    }
                    env.request_redraw();
                }
                KeyCode::Left => {
                    self.prev_track();
                    env.request_redraw();
                }
                KeyCode::Right => {
                    self.next_track();
                    env.request_redraw();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {
        PcSpeaker::stop_tone();
    }

    fn render(&self, canvas: &mut Canvas) {
        let w = canvas.width;
        let h = canvas.height;

        // Dark media studio background (#0B0F19)
        canvas.draw_rect_fill(Rect::new(0, 0, w, h), Color32::rgb(0x0B, 0x0F, 0x19));

        // Top Header Banner
        canvas.draw_rect_fill(Rect::new(0, 0, w, 32), Color32::rgb(0x11, 0x18, 0x27));
        canvas.draw_hline(0, 31, w, Color32::rgb(0x1F, 0x29, 0x37));
        canvas.draw_text(12, 8, "INTEGRATE OS MEDIA PLAYER", Color32::rgb(0x38, 0xBD, 0xF8), None);
        canvas.draw_text((w as i32) - 170, 8, "[VFS: /mnt/ssd/videos]", Color32::rgb(0x94, 0xA3, 0xB8), None);

        // =====================================================================
        // VIEWPORT SCREEN (240x136 16:9 Cinema Box at x=20, y=42)
        // =====================================================================
        let vp_rect = Rect::new(20, 42, 240, 136);
        canvas.draw_rect_fill(vp_rect, Color32::BLACK);
        canvas.draw_rect_outline(vp_rect, Color32::rgb(0x22, 0xD3, 0xEE));

        let current_track = &self.tracks[self.current_track_idx];

        if current_track.media_type == MediaType::Video {
            // Render video frame pixels scaled 2x
            let vw = self.video_width;
            let vh = self.video_height;
            for vy in 0..vh.min(68) {
                for vx in 0..vw.min(120) {
                    let c565 = self.pixel_buffer[vy * vw + vx];
                    let r = (((c565 >> 11) & 0x1F) * 255 / 31) as u8;
                    let g = (((c565 >> 5) & 0x3F) * 255 / 63) as u8;
                    let b = ((c565 & 0x1F) * 255 / 31) as u8;
                    let col = Color32::rgb(r, g, b);

                    // 2x blit to screen
                    let px = 20 + (vx as i32 * 2);
                    let py = 42 + (vy as i32 * 2);
                    canvas.draw_pixel(px, py, col);
                    canvas.draw_pixel(px + 1, py, col);
                    canvas.draw_pixel(px, py + 1, col);
                    canvas.draw_pixel(px + 1, py + 1, col);
                }
            }
        } else {
            // Audio spectrum analyzer visualization
            canvas.draw_text(60, 60, "NOW PLAYING AUDIO", Color32::rgb(0x34, 0xD3, 0x99), None);
            canvas.draw_text(45, 78, current_track.title, Color32::WHITE, None);

            // Equalizer bars
            let bar_base_y = 168;
            for i in 0..16 {
                let bx = 36 + (i as i32 * 13);
                let bh = self.spectrum_heights[i] as i32;
                let bar_color = if bh > 25 {
                    Color32::rgb(0xEF, 0x44, 0x44)
                } else if bh > 15 {
                    Color32::rgb(0xFB, 0xBF, 0x24)
                } else {
                    Color32::rgb(0x10, 0xB9, 0x81)
                };
                canvas.draw_rect_fill(Rect::new(bx, bar_base_y - bh, 9, bh as u32), bar_color);
            }
        }

        // Title and Format Badge
        canvas.draw_text(20, 186, current_track.title, Color32::WHITE, None);
        let badge = if current_track.media_type == MediaType::Video {
            format!("[{}x{} RGB565 @ {}FPS]", self.video_width, self.video_height, if self.frame_interval_ms > 0 { 1000 / self.frame_interval_ms } else { 6 })
        } else {
            String::from("[PCM SYNTH 44.1kHz]")
        };
        canvas.draw_text(20, 204, &badge, Color32::rgb(0x38, 0xBD, 0xF8), None);

        // Frame / Time progress text
        let time_str = if current_track.media_type == MediaType::Video {
            let sec_cur = if self.total_frames > 0 {
                (self.current_frame * current_track.duration_seconds as usize) / self.total_frames
            } else {
                0
            };
            format!("00:{:02} / 00:{:02} (F#{}/{})", sec_cur, current_track.duration_seconds, self.current_frame + 1, self.total_frames)
        } else {
            format!("00:04 / 00:{:02} [ACTIVE TONE]", current_track.duration_seconds)
        };
        canvas.draw_text(240, 204, &time_str, Color32::rgb(0x94, 0xA3, 0xB8), None);

        // =====================================================================
        // PROGRESS / SCRUBBER BAR (x=20, y=222, width=350)
        // =====================================================================
        let bar_w = 350u32;
        canvas.draw_rect_fill(Rect::new(20, 222, bar_w, 8), Color32::rgb(0x1F, 0x29, 0x37));
        let progress_w = if self.total_frames > 0 {
            ((self.current_frame as u32 * bar_w) / (self.total_frames as u32).max(1)).clamp(0, bar_w)
        } else {
            0
        };
        canvas.draw_rect_fill(Rect::new(20, 222, progress_w, 8), Color32::rgb(0x06, 0xB6, 0xD4));
        canvas.draw_rect_outline(Rect::new(20, 222, bar_w, 8), Color32::rgb(0x37, 0x41, 0x51));

        // =====================================================================
        // TRANSPORT BUTTONS (Prev, Play/Pause, Stop, Next) with Animations
        // =====================================================================
        let btns = [
            (20, 54, "|<< Prev", 0),
            (80, 66, if self.state == PlaybackState::Playing { "|| Pause" } else { "> Play" }, 1),
            (152, 54, "[#] Stop", 2),
            (212, 54, "Next >>|", 3),
        ];

        for &(bx, bw, label, btn_id) in &btns {
            let is_hover = self.hovered_button == Some(btn_id);
            let is_pressed = self.pressed_button == Some(btn_id);

            let offset = if is_pressed { 1 } else { 0 };
            let bg = if is_pressed {
                Color32::rgb(0x0E, 0x3A, 0x56)
            } else if is_hover {
                Color32::rgb(0x16, 0x4E, 0x63)
            } else {
                Color32::rgb(0x1F, 0x29, 0x37)
            };

            let border = if is_hover {
                Color32::rgb(0x22, 0xD3, 0xEE)
            } else {
                Color32::rgb(0x37, 0x41, 0x51)
            };

            canvas.draw_rect_fill(Rect::new(bx + offset, 240 + offset, bw, 26), bg);
            canvas.draw_rect_outline(Rect::new(bx + offset, 240 + offset, bw, 26), border);
            canvas.draw_text(bx + 8 + offset, 245 + offset, label, Color32::WHITE, None);
        }

        // Volume Badge
        canvas.draw_rect_fill(Rect::new(278, 240, 92, 26), Color32::rgb(0x11, 0x18, 0x27));
        canvas.draw_rect_outline(Rect::new(278, 240, 92, 26), Color32::rgb(0x10, 0xB9, 0x81));
        canvas.draw_text(286, 245, "Vol: 100%", Color32::rgb(0x34, 0xD3, 0x99), None);

        // =====================================================================
        // RIGHT PLAYLIST PANEL (x=384..550)
        // =====================================================================
        let pl_rect = Rect::new(384, 42, w.saturating_sub(394), 316);
        canvas.draw_rect_fill(pl_rect, Color32::rgb(0x11, 0x18, 0x27));
        canvas.draw_rect_outline(pl_rect, Color32::rgb(0x37, 0x41, 0x51));
        canvas.draw_text(392, 48, "PLAYLIST TRACKS", Color32::rgb(0x38, 0xBD, 0xF8), None);

        let mut ty = 66;
        for (i, t) in self.tracks.iter().enumerate() {
            let is_sel = i == self.current_track_idx;
            let item_bg = if is_sel {
                Color32::rgb(0x1E, 0x3A, 0x5F)
            } else {
                Color32::rgb(0x17, 0x20, 0x33)
            };
            canvas.draw_rect_fill(Rect::new(388, ty, w.saturating_sub(402), 34), item_bg);
            if is_sel {
                canvas.draw_rect_outline(Rect::new(388, ty, w.saturating_sub(402), 34), Color32::rgb(0x60, 0xA5, 0xFA));
            }

            let icon_str = if t.media_type == MediaType::Video { "[VID]" } else { "[AUD]" };
            let mut line = format!("{} {}", icon_str, t.title);
            if line.len() > 22 {
                line.truncate(22);
            }
            canvas.draw_text(392, ty + 9, &line, if is_sel { Color32::WHITE } else { Color32::rgb(0xCB, 0xD5, 0xE1) }, None);
            ty += 38;
        }

        // =====================================================================
        // FOOTER STATUS BAR
        // =====================================================================
        canvas.draw_rect_fill(Rect::new(0, (h.saturating_sub(26)) as i32, w, 26), Color32::rgb(0x0F, 0x17, 0x2A));
        canvas.draw_hline(0, (h.saturating_sub(26)) as i32, w, Color32::rgb(0x1E, 0x29, 0x3B));
        canvas.draw_text(12, (h.saturating_sub(20)) as i32, &self.status_banner, Color32::rgb(0x38, 0xBD, 0xF8), None);
    }
}
