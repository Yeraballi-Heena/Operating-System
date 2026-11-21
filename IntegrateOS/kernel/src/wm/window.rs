//! Managed Window Representation & Geometry
//!
//! Encapsulates window bounds, title, state, flags, offscreen client surface buffer,
//! decoration metrics, and precise hit-testing logic.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::graphics::font::{get_glyph, FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;
use crate::wm::theme::*;
use crate::wm::types::*;

pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub bounds: Rect,
    pub flags: WindowFlags,
    pub state: WindowState,
    pub saved_bounds: Rect,
    pub content_buffer: Vec<u32>,
    pub active: bool,
    pub min_width: u32,
    pub min_height: u32,
}

impl Window {
    /// Creates a new managed window with allocated client buffer.
    pub fn new(
        id: WindowId,
        title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        flags: WindowFlags,
    ) -> Self {
        let min_w = 120;
        let min_h = TITLE_BAR_HEIGHT + BORDER_WIDTH * 2 + 30;
        let width = width.max(min_w);
        let height = height.max(min_h);
        let bounds = Rect::new(x, y, width, height);

        let mut win = Self {
            id,
            title: String::from(title),
            bounds,
            flags,
            state: WindowState::Normal,
            saved_bounds: bounds,
            content_buffer: Vec::new(),
            active: false,
            min_width: min_w,
            min_height: min_h,
        };

        win.resize_buffer();
        win.fill_client(CLIENT_BG_DEFAULT);
        win
    }

    /// Computes the inner client rectangle where content is rendered.
    pub fn client_rect(&self) -> Rect {
        if self.flags.contains(WindowFlags::BORDERLESS) {
            self.bounds
        } else {
            let cx = self.bounds.x + BORDER_WIDTH as i32;
            let cy = self.bounds.y + TITLE_BAR_HEIGHT as i32;
            let cw = self.bounds.width.saturating_sub(BORDER_WIDTH * 2);
            let ch = self
                .bounds
                .height
                .saturating_sub(TITLE_BAR_HEIGHT + BORDER_WIDTH);
            Rect::new(cx, cy, cw, ch)
        }
    }

    /// Returns the client surface dimensions `(width, height)`.
    pub fn client_size(&self) -> (u32, u32) {
        let cr = self.client_rect();
        (cr.width, cr.height)
    }

    /// Computes the title bar rectangle.
    pub fn title_bar_rect(&self) -> Rect {
        if self.flags.contains(WindowFlags::BORDERLESS) {
            Rect::new(0, 0, 0, 0)
        } else {
            let tx = self.bounds.x + BORDER_WIDTH as i32;
            let ty = self.bounds.y + BORDER_WIDTH as i32;
            let tw = self.bounds.width.saturating_sub(BORDER_WIDTH * 2);
            let th = TITLE_BAR_HEIGHT.saturating_sub(BORDER_WIDTH);
            Rect::new(tx, ty, tw, th)
        }
    }

    /// Computes the close button bounds.
    pub fn close_button_rect(&self) -> Rect {
        let by = self.bounds.y + ((TITLE_BAR_HEIGHT - BUTTON_SIZE) / 2) as i32;
        let bx = self.bounds.right() - (BORDER_WIDTH + BUTTON_MARGIN_RIGHT + BUTTON_SIZE) as i32;
        Rect::new(bx, by, BUTTON_SIZE, BUTTON_SIZE)
    }

    /// Computes the maximize button bounds.
    pub fn maximize_button_rect(&self) -> Rect {
        let close_rect = self.close_button_rect();
        let bx = close_rect.x - (BUTTON_SPACING + BUTTON_SIZE) as i32;
        Rect::new(bx, close_rect.y, BUTTON_SIZE, BUTTON_SIZE)
    }

    /// Computes the minimize button bounds.
    pub fn minimize_button_rect(&self) -> Rect {
        let max_rect = self.maximize_button_rect();
        let bx = max_rect.x - (BUTTON_SPACING + BUTTON_SIZE) as i32;
        Rect::new(bx, max_rect.y, BUTTON_SIZE, BUTTON_SIZE)
    }

    /// Reallocates the client content buffer to match current client rectangle dimensions.
    pub fn resize_buffer(&mut self) {
        let (cw, ch) = self.client_size();
        let total = (cw as usize) * (ch as usize);
        self.content_buffer = vec![CLIENT_BG_DEFAULT; total];
    }

    /// Fills the entire client surface with a solid color.
    pub fn fill_client(&mut self, color: u32) {
        for pixel in self.content_buffer.iter_mut() {
            *pixel = color;
        }
    }

    /// Sets a pixel in client coordinates `(cx, cy)`.
    pub fn set_client_pixel(&mut self, cx: u32, cy: u32, color: u32) {
        let (cw, ch) = self.client_size();
        if cx < cw && cy < ch {
            let idx = (cy as usize) * (cw as usize) + (cx as usize);
            if idx < self.content_buffer.len() {
                self.content_buffer[idx] = color;
            }
        }
    }

    /// Gets a pixel in client coordinates `(cx, cy)`.
    pub fn get_client_pixel(&self, cx: u32, cy: u32) -> Option<u32> {
        let (cw, ch) = self.client_size();
        if cx < cw && cy < ch {
            let idx = (cy as usize) * (cw as usize) + (cx as usize);
            self.content_buffer.get(idx).copied()
        } else {
            None
        }
    }

    /// Draws a solid rectangle in client surface coordinates.
    pub fn draw_client_rect(&mut self, rect: Rect, color: u32) {
        let (cw, ch) = self.client_size();
        let r_left = rect.left().clamp(0, cw as i32) as u32;
        let r_right = rect.right().clamp(0, cw as i32) as u32;
        let r_top = rect.top().clamp(0, ch as i32) as u32;
        let r_bottom = rect.bottom().clamp(0, ch as i32) as u32;

        for y in r_top..r_bottom {
            for x in r_left..r_right {
                let idx = (y as usize) * (cw as usize) + (x as usize);
                if idx < self.content_buffer.len() {
                    self.content_buffer[idx] = color;
                }
            }
        }
    }

    /// Renders text into the client buffer using the 8x16 font.
    pub fn draw_client_string(&mut self, start_x: u32, start_y: u32, text: &str, fg: u32, bg: u32) {
        let mut cur_x = start_x;
        let mut cur_y = start_y;
        let (cw, ch) = self.client_size();

        for b in text.bytes() {
            if b == b'\n' {
                cur_x = start_x;
                cur_y += FONT_HEIGHT as u32;
                continue;
            }

            let glyph = get_glyph(b as char);
            for row in 0..FONT_HEIGHT {
                let py = cur_y + row as u32;
                if py >= ch {
                    continue;
                }
                let bits = glyph[row];
                for col in 0..FONT_WIDTH {
                    let px = cur_x + col as u32;
                    if px >= cw {
                        continue;
                    }
                    let is_fg = (bits & (0x80 >> col)) != 0;
                    self.set_client_pixel(px, py, if is_fg { fg } else { bg });
                }
            }

            cur_x += FONT_WIDTH as u32;
        }
    }

    /// Performs a regional hit-test against this window at screen coordinate `(px, py)`.
    pub fn hit_test(&self, px: i32, py: i32) -> HitTestResult {
        if self.state == WindowState::Minimized || self.state == WindowState::Hidden {
            return HitTestResult::Nowhere;
        }

        if !self.bounds.contains(px, py) {
            return HitTestResult::Nowhere;
        }

        if self.flags.contains(WindowFlags::BORDERLESS) {
            return HitTestResult::ClientArea;
        }

        // 1. Control buttons
        if self.flags.contains(WindowFlags::CLOSABLE) && self.close_button_rect().contains(px, py) {
            return HitTestResult::CloseButton;
        }
        if self.flags.contains(WindowFlags::MAXIMIZABLE)
            && self.maximize_button_rect().contains(px, py)
        {
            return HitTestResult::MaximizeButton;
        }
        if self.flags.contains(WindowFlags::MINIMIZABLE)
            && self.minimize_button_rect().contains(px, py)
        {
            return HitTestResult::MinimizeButton;
        }

        // 2. Title bar
        if self.title_bar_rect().contains(px, py) {
            return HitTestResult::TitleBar;
        }

        // 3. Client area
        if self.client_rect().contains(px, py) {
            return HitTestResult::ClientArea;
        }

        // 4. Resizing borders & corners
        if self.flags.contains(WindowFlags::RESIZABLE) {
            let handle = RESIZE_HANDLE_SIZE as i32;
            let left = px < self.bounds.x + handle;
            let right = px >= self.bounds.right() - handle;
            let top = py < self.bounds.y + handle;
            let bottom = py >= self.bounds.bottom() - handle;

            if top && left {
                return HitTestResult::ResizeBorder(ResizeEdge::TopLeft);
            }
            if top && right {
                return HitTestResult::ResizeBorder(ResizeEdge::TopRight);
            }
            if bottom && left {
                return HitTestResult::ResizeBorder(ResizeEdge::BottomLeft);
            }
            if bottom && right {
                return HitTestResult::ResizeBorder(ResizeEdge::BottomRight);
            }
            if left {
                return HitTestResult::ResizeBorder(ResizeEdge::Left);
            }
            if right {
                return HitTestResult::ResizeBorder(ResizeEdge::Right);
            }
            if top {
                return HitTestResult::ResizeBorder(ResizeEdge::Top);
            }
            if bottom {
                return HitTestResult::ResizeBorder(ResizeEdge::Bottom);
            }
        }

        HitTestResult::ClientArea
    }
}
