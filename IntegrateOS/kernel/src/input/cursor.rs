//! Virtual Mouse Cursor & Sprite Renderer
//!
//! Tracks screen-space cursor coordinates, enforces display boundary clamping,
//! and provides the standard GUI arrow cursor sprite with boundary clipping.

pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 19;

/// Standard arrow cursor bitmap:
/// - 0: Transparent
/// - 1: Black outline (0xFF000000)
/// - 2: White body (0xFFFFFFFF)
pub const CURSOR_SPRITE: [[u8; CURSOR_WIDTH]; CURSOR_HEIGHT] = [
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0],
    [1, 2, 2, 1, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 1, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0],
];

/// Screen-space virtual mouse cursor tracker.
#[derive(Debug, Clone, Copy)]
pub struct VirtualCursor {
    pub x: i32,
    pub y: i32,
    pub screen_width: i32,
    pub screen_height: i32,
    pub visible: bool,
}

impl VirtualCursor {
    pub const fn new(width: i32, height: i32) -> Self {
        Self {
            x: width / 2,
            y: height / 2,
            screen_width: width,
            screen_height: height,
            visible: true,
        }
    }

    /// Moves the cursor by `(dx, dy)`, clamping strictly to screen boundaries.
    pub fn move_relative(&mut self, dx: i32, dy: i32) {
        self.x = (self.x + dx).clamp(0, self.screen_width.saturating_sub(1));
        self.y = (self.y + dy).clamp(0, self.screen_height.saturating_sub(1));
    }

    /// Sets the cursor to an absolute screen coordinate, clamped to screen boundaries.
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x.clamp(0, self.screen_width.saturating_sub(1));
        self.y = y.clamp(0, self.screen_height.saturating_sub(1));
    }

    /// Updates the screen boundary limits.
    pub fn set_bounds(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
        self.set_position(self.x, self.y);
    }

    /// Returns the current clamped `(x, y)` coordinate.
    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    /// Renders the cursor sprite by invoking `draw_pixel(x, y, argb_color)` for each opaque pixel.
    pub fn render<F>(&self, mut draw_pixel: F)
    where
        F: FnMut(i32, i32, u32),
    {
        if !self.visible {
            return;
        }

        for row in 0..CURSOR_HEIGHT {
            let py = self.y + row as i32;
            if py < 0 || py >= self.screen_height {
                continue;
            }

            for col in 0..CURSOR_WIDTH {
                let px = self.x + col as i32;
                if px < 0 || px >= self.screen_width {
                    continue;
                }

                match CURSOR_SPRITE[row][col] {
                    1 => draw_pixel(px, py, 0xFF00_0000), // Black outline
                    2 => draw_pixel(px, py, 0xFFFF_FFFF), // White interior
                    _ => {}                               // Transparent
                }
            }
        }
    }
}
