//! Graphical Framebuffer Console
//!
//! Provides a text terminal emulator rendered on top of the graphics canvas using
//! the 8x16 monospace font, supporting line wrapping, newline handling, scrolling,
//! and ANSI/custom colors.

use alloc::vec;
use alloc::vec::Vec;

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;

pub struct FramebufferConsole {
    pub bounds: Rect,
    pub cols: usize,
    pub rows: usize,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub fg: Color32,
    pub bg: Color32,
    pub buffer: Vec<Vec<(char, Color32, Color32)>>,
}

impl FramebufferConsole {
    /// Creates a graphical console inside the designated rectangle.
    pub fn new(bounds: Rect, fg: Color32, bg: Color32) -> Self {
        let cols = (bounds.width as usize) / FONT_WIDTH;
        let rows = (bounds.height as usize) / FONT_HEIGHT;

        let empty_row = vec![(' ', fg, bg); cols];
        let buffer = vec![empty_row; rows];

        Self {
            bounds,
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            fg,
            bg,
            buffer,
        }
    }

    /// Sets active foreground and background colors.
    pub fn set_colors(&mut self, fg: Color32, bg: Color32) {
        self.fg = fg;
        self.bg = bg;
    }

    /// Clears the console area on the canvas and resets the cursor.
    pub fn clear(&mut self, canvas: &mut Canvas) {
        canvas.draw_filled_rect(self.bounds, self.bg);
        for row in self.buffer.iter_mut() {
            row.fill((' ', self.fg, self.bg));
        }
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    /// Writes a single ASCII byte/character to the graphical console.
    pub fn write_char(&mut self, canvas: &mut Canvas, ch: char) {
        if ch == '\n' {
            self.newline(canvas);
            return;
        }

        if ch == '\r' {
            self.cursor_col = 0;
            return;
        }

        if ch == '\t' {
            let next_tab = (self.cursor_col + 4) & !3;
            while self.cursor_col < next_tab && self.cursor_col < self.cols {
                self.write_char(canvas, ' ');
            }
            return;
        }

        if self.cursor_col >= self.cols {
            self.newline(canvas);
        }

        // Store into internal text buffer
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.buffer[self.cursor_row][self.cursor_col] = (ch, self.fg, self.bg);
        }

        // Draw character directly onto canvas
        let px = self.bounds.x + (self.cursor_col * FONT_WIDTH) as i32;
        let py = self.bounds.y + (self.cursor_row * FONT_HEIGHT) as i32;
        canvas.draw_char(px, py, ch, self.fg, Some(self.bg));

        self.cursor_col += 1;
    }

    /// Advances the cursor to the next line, scrolling if at the bottom.
    pub fn newline(&mut self, canvas: &mut Canvas) {
        self.cursor_col = 0;
        if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        } else {
            self.scroll(canvas);
        }
    }

    /// Scrolls the console buffer up by one row and redraws the text.
    pub fn scroll(&mut self, canvas: &mut Canvas) {
        // Shift rows up in buffer
        for r in 1..self.rows {
            self.buffer[r - 1] = self.buffer[r].clone();
        }
        self.buffer[self.rows - 1] = vec![(' ', self.fg, self.bg); self.cols];

        // Redraw whole console region
        canvas.draw_filled_rect(self.bounds, self.bg);
        for r in 0..self.rows {
            let py = self.bounds.y + (r * FONT_HEIGHT) as i32;
            for c in 0..self.cols {
                let (ch, fg, bg) = self.buffer[r][c];
                if ch != ' ' || bg != self.bg {
                    let px = self.bounds.x + (c * FONT_WIDTH) as i32;
                    canvas.draw_char(px, py, ch, fg, Some(bg));
                }
            }
        }
    }

    /// Writes a full string slice to the graphical console.
    pub fn write_str(&mut self, canvas: &mut Canvas, s: &str) {
        for ch in s.chars() {
            self.write_char(canvas, ch);
        }
    }
}
