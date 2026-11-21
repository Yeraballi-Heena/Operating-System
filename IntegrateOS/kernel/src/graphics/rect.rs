//! 2D Geometric Primitives & Boundary Clipping
//!
//! Provides `Point`, `Size`, and `Rect` with intersection, union, and clipping logic.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle.
    #[inline(always)]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Left horizontal coordinate.
    #[inline(always)]
    pub const fn left(&self) -> i32 {
        self.x
    }

    /// Top vertical coordinate.
    #[inline(always)]
    pub const fn top(&self) -> i32 {
        self.y
    }

    /// Right horizontal bound (exclusive).
    #[inline(always)]
    pub const fn right(&self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    /// Bottom vertical bound (exclusive).
    #[inline(always)]
    pub const fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    /// Returns true if the rectangle has zero width or height.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Tests whether point `(px, py)` is within the rectangle bounds.
    #[inline]
    pub const fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    /// Tests whether this rectangle intersects with another rectangle.
    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Computes the intersection of two rectangles, returning `None` if they do not overlap.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 > x1 && y2 > y1 {
            Some(Rect {
                x: x1,
                y: y1,
                width: (x2 - x1) as u32,
                height: (y2 - y1) as u32,
            })
        } else {
            None
        }
    }

    /// Computes the bounding union containing both rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }

        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());

        Rect {
            x: x1,
            y: y1,
            width: (x2 - x1) as u32,
            height: (y2 - y1) as u32,
        }
    }

    /// Insets the rectangle by `(dx, dy)`.
    pub fn inset(&self, dx: i32, dy: i32) -> Rect {
        let new_x = self.x + dx;
        let new_y = self.y + dy;
        let new_w = (self.width as i32 - 2 * dx).max(0) as u32;
        let new_h = (self.height as i32 - 2 * dy).max(0) as u32;
        Rect::new(new_x, new_y, new_w, new_h)
    }

    /// Offsets the rectangle by `(dx, dy)`.
    #[inline(always)]
    pub const fn offset(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.width, self.height)
    }
}
