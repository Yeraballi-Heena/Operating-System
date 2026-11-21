//! Double-Buffered Graphics Canvas & 2D Drawing Primitives
//!
//! Provides `Canvas` implementing:
//! - Double buffering (heap-allocated back buffer presented to physical VRAM)
//! - Strict boundary clipping (`clip_rect`)
//! - Drawing primitives: pixels, Bresenham lines, outline/filled rectangles,
//!   midpoint circles, horizontal/vertical color gradients
//! - 8x16 bitmap font text and glyph rendering

use alloc::vec;
use alloc::vec::Vec;

use crate::graphics::color::Color32;
use crate::graphics::font::{get_glyph, FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pub clip_rect: Rect,
    pub back_buffer: Vec<u32>,
    pub vram_ptr: *mut u32,
    pub damage_rect: Option<Rect>,
}

// SAFETY: Canvas owns its back_buffer and manages raw access to VRAM in Ring 0.
unsafe impl Send for Canvas {}
unsafe impl Sync for Canvas {}

impl Canvas {
    /// Creates a double-buffered canvas for the specified video mode and VRAM physical pointer.
    pub fn new(width: u32, height: u32, pitch: u32, bpp: u8, vram_phys_addr: u64) -> Self {
        let total_pixels = (width as usize) * (height as usize);
        let back_buffer = vec![0u32; total_pixels];
        let clip_rect = Rect::new(0, 0, width, height);
        let vram_ptr = vram_phys_addr as *mut u32;

        Self {
            width,
            height,
            pitch,
            bpp,
            clip_rect,
            back_buffer,
            vram_ptr,
            damage_rect: Some(clip_rect),
        }
    }

    /// Sets the active clipping rectangle, clamping it to canvas dimensions.
    pub fn set_clip_rect(&mut self, rect: Rect) {
        let screen = Rect::new(0, 0, self.width, self.height);
        self.clip_rect = rect.intersection(&screen).unwrap_or(Rect::new(0, 0, 0, 0));
    }

    /// Resets the clipping rectangle to the entire screen.
    pub fn reset_clip_rect(&mut self) {
        self.clip_rect = Rect::new(0, 0, self.width, self.height);
    }

    /// Marks a region as damaged/dirty for optimized presentation.
    pub fn mark_dirty(&mut self, rect: Rect) {
        let screen = Rect::new(0, 0, self.width, self.height);
        if let Some(clipped) = rect.intersection(&screen) {
            self.damage_rect = match self.damage_rect {
                Some(existing) => Some(existing.union(&clipped)),
                None => Some(clipped),
            };
        }
    }

    /// Clears the entire back buffer with the specified color.
    pub fn clear(&mut self, color: Color32) {
        let raw = color.to_raw();
        self.back_buffer.fill(raw);
        self.mark_dirty(Rect::new(0, 0, self.width, self.height));
    }

    /// Draws a single pixel at `(x, y)` clipped against `clip_rect`.
    #[inline]
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color32) {
        if !self.clip_rect.contains(x, y) {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);
        if idx < self.back_buffer.len() {
            self.back_buffer[idx] = color.to_raw();
            self.mark_dirty(Rect::new(x, y, 1, 1));
        }
    }

    /// Draws a pixel with alpha blending over the existing back buffer content.
    #[inline]
    pub fn draw_pixel_blend(&mut self, x: i32, y: i32, color: Color32) {
        if !self.clip_rect.contains(x, y) {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);
        if idx < self.back_buffer.len() {
            let existing = Color32::from_u32(self.back_buffer[idx]);
            let blended = color.alpha_blend(existing);
            self.back_buffer[idx] = blended.to_raw();
            self.mark_dirty(Rect::new(x, y, 1, 1));
        }
    }

    /// Reads back a pixel from the back buffer.
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<Color32> {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return None;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.back_buffer.get(idx).map(|&val| Color32::from_u32(val))
    }

    /// Draws an optimized horizontal line.
    pub fn draw_hline(&mut self, x: i32, y: i32, length: u32, color: Color32) {
        if y < self.clip_rect.y || y >= self.clip_rect.bottom() || length == 0 {
            return;
        }

        let x_start = x.max(self.clip_rect.x);
        let x_end = (x + length as i32).min(self.clip_rect.right());

        if x_end <= x_start {
            return;
        }

        let raw = color.to_raw();
        let row_start = (y as usize) * (self.width as usize);
        let start_idx = row_start + (x_start as usize);
        let end_idx = row_start + (x_end as usize);

        self.back_buffer[start_idx..end_idx].fill(raw);
        self.mark_dirty(Rect::new(x_start, y, (x_end - x_start) as u32, 1));
    }

    /// Draws an optimized vertical line.
    pub fn draw_vline(&mut self, x: i32, y: i32, height: u32, color: Color32) {
        if x < self.clip_rect.x || x >= self.clip_rect.right() || height == 0 {
            return;
        }

        let y_start = y.max(self.clip_rect.y);
        let y_end = (y + height as i32).min(self.clip_rect.bottom());

        if y_end <= y_start {
            return;
        }

        let raw = color.to_raw();
        let stride = self.width as usize;
        let mut idx = (y_start as usize) * stride + (x as usize);

        for _ in y_start..y_end {
            self.back_buffer[idx] = raw;
            idx += stride;
        }

        self.mark_dirty(Rect::new(x, y_start, 1, (y_end - y_start) as u32));
    }

    /// Draws an arbitrary line using Bresenham's line algorithm.
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Color32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.draw_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws an outline rectangle.
    pub fn draw_rect(&mut self, rect: Rect, color: Color32) {
        if rect.is_empty() {
            return;
        }
        self.draw_hline(rect.x, rect.y, rect.width, color);
        self.draw_hline(rect.x, rect.bottom() - 1, rect.width, color);
        self.draw_vline(rect.x, rect.y, rect.height, color);
        self.draw_vline(rect.right() - 1, rect.y, rect.height, color);
    }

    /// Alias for drawing an outline rectangle.
    #[inline(always)]
    pub fn draw_rect_outline(&mut self, rect: Rect, color: Color32) {
        self.draw_rect(rect, color);
    }

    /// Draws a filled rectangle.
    pub fn draw_filled_rect(&mut self, rect: Rect, color: Color32) {
        if let Some(clipped) = rect.intersection(&self.clip_rect) {
            for y in clipped.y..clipped.bottom() {
                self.draw_hline(clipped.x, y, clipped.width, color);
            }
        }
    }

    /// Alias for drawing a filled rectangle.
    #[inline(always)]
    pub fn draw_rect_fill(&mut self, rect: Rect, color: Color32) {
        self.draw_filled_rect(rect, color);
    }

    /// Draws an outline circle using Midpoint Circle algorithm.
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color32) {
        if radius <= 0 {
            self.draw_pixel(cx, cy, color);
            return;
        }

        let mut x = 0;
        let mut y = radius;
        let mut d = 1 - radius;

        let mut plot_8 = |cx: i32, cy: i32, x: i32, y: i32| {
            self.draw_pixel(cx + x, cy + y, color);
            self.draw_pixel(cx - x, cy + y, color);
            self.draw_pixel(cx + x, cy - y, color);
            self.draw_pixel(cx - x, cy - y, color);
            self.draw_pixel(cx + y, cy + x, color);
            self.draw_pixel(cx - y, cy + x, color);
            self.draw_pixel(cx + y, cy - x, color);
            self.draw_pixel(cx - y, cy - x, color);
        };

        plot_8(cx, cy, x, y);

        while x < y {
            x += 1;
            if d < 0 {
                d += 2 * x + 1;
            } else {
                y -= 1;
                d += 2 * (x - y) + 1;
            }
            plot_8(cx, cy, x, y);
        }
    }

    /// Draws a filled circle using Midpoint horizontal span lines.
    pub fn draw_filled_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color32) {
        if radius <= 0 {
            self.draw_pixel(cx, cy, color);
            return;
        }

        let mut x = 0;
        let mut y = radius;
        let mut d = 1 - radius;

        let mut draw_spans = |cx: i32, cy: i32, x: i32, y: i32| {
            self.draw_hline(cx - x, cy + y, (2 * x + 1) as u32, color);
            self.draw_hline(cx - x, cy - y, (2 * x + 1) as u32, color);
            self.draw_hline(cx - y, cy + x, (2 * y + 1) as u32, color);
            self.draw_hline(cx - y, cy - x, (2 * y + 1) as u32, color);
        };

        draw_spans(cx, cy, x, y);

        while x < y {
            x += 1;
            if d < 0 {
                d += 2 * x + 1;
            } else {
                y -= 1;
                d += 2 * (x - y) + 1;
            }
            draw_spans(cx, cy, x, y);
        }
    }

    /// Draws a horizontal linear color gradient.
    pub fn draw_gradient_h(&mut self, rect: Rect, start: Color32, end: Color32) {
        if let Some(clipped) = rect.intersection(&self.clip_rect) {
            let total_w = rect.width.max(1);
            for x in clipped.x..clipped.right() {
                let t = (x - rect.x) as u32;
                let col = Color32::lerp(start, end, t, total_w);
                self.draw_vline(x, clipped.y, clipped.height, col);
            }
        }
    }

    /// Draws a vertical linear color gradient.
    pub fn draw_gradient_v(&mut self, rect: Rect, start: Color32, end: Color32) {
        if let Some(clipped) = rect.intersection(&self.clip_rect) {
            let total_h = rect.height.max(1);
            for y in clipped.y..clipped.bottom() {
                let t = (y - rect.y) as u32;
                let col = Color32::lerp(start, end, t, total_h);
                self.draw_hline(clipped.x, y, clipped.width, col);
            }
        }
    }

    /// Renders a single 8x16 bitmap character at `(x, y)`.
    pub fn draw_char(&mut self, x: i32, y: i32, ch: char, fg: Color32, bg: Option<Color32>) {
        let glyph = get_glyph(ch);

        for dy in 0..FONT_HEIGHT {
            let row = glyph[dy];
            for dx in 0..FONT_WIDTH {
                let pixel_set = (row & (1 << (7 - dx))) != 0;
                let px = x + dx as i32;
                let py = y + dy as i32;

                if pixel_set {
                    self.draw_pixel(px, py, fg);
                } else if let Some(bg_color) = bg {
                    self.draw_pixel(px, py, bg_color);
                }
            }
        }
    }

    /// Renders a string of text using the 8x16 font with newline support.
    pub fn draw_text(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        fg: Color32,
        bg: Option<Color32>,
    ) {
        let mut cur_x = x;
        let mut cur_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = x;
                cur_y += FONT_HEIGHT as i32;
                continue;
            }
            self.draw_char(cur_x, cur_y, ch, fg, bg);
            cur_x += FONT_WIDTH as i32;
        }
    }

    /// Flips the entire back buffer to physical VRAM using 64-bit block writes.
    pub fn present(&mut self) {
        if self.vram_ptr.is_null() {
            return;
        }

        let total_words = (self.width as usize * self.height as usize) / 2;
        let src = self.back_buffer.as_ptr() as *const u64;
        let dst = self.vram_ptr as *mut u64;

        for i in 0..total_words {
            // SAFETY: src is valid in kernel heap; dst is valid mapped VRAM linear framebuffer.
            unsafe {
                core::ptr::write_volatile(dst.add(i), core::ptr::read(src.add(i)));
            }
        }

        self.damage_rect = None;
    }

    /// Flips only damaged/dirty regions to VRAM to conserve memory bandwidth.
    pub fn present_dirty(&mut self) {
        if let Some(dirty) = self.damage_rect {
            self.present_rect(dirty);
            self.damage_rect = None;
        }
    }

    /// Flips a specific rectangular sub-region to physical VRAM.
    pub fn present_rect(&mut self, rect: Rect) {
        if self.vram_ptr.is_null() {
            return;
        }

        let screen = Rect::new(0, 0, self.width, self.height);
        let Some(clipped) = rect.intersection(&screen) else {
            return;
        };

        for y in clipped.y..clipped.bottom() {
            let row_offset = (y as usize) * (self.width as usize) + (clipped.x as usize);
            let src = unsafe { self.back_buffer.as_ptr().add(row_offset) };
            let dst = unsafe { self.vram_ptr.add(row_offset) };

            for x in 0..(clipped.width as usize) {
                // SAFETY: Writing row pixels into hardware framebuffer.
                unsafe {
                    core::ptr::write_volatile(dst.add(x), core::ptr::read(src.add(x)));
                }
            }
        }
    }
}
