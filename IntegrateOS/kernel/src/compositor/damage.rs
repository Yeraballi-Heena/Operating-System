//! Damage Tracking & Dirty Region Optimization
//!
//! Tracks changed display regions, merges contiguous and overlapping dirty rectangles,
//! and computes bounded flush geometries to minimize memory bandwidth to physical VRAM.

use alloc::vec::Vec;

use crate::graphics::rect::Rect;

pub const MAX_DIRTY_RECTS: usize = 16;

/// Bounded damage region tracking dirty rectangles across frames.
#[derive(Debug, Clone)]
pub struct DamageRegion {
    rects: Vec<Rect>,
    bounds: Option<Rect>,
    screen_bounds: Rect,
}

impl DamageRegion {
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            rects: Vec::new(),
            bounds: None,
            screen_bounds: Rect::new(0, 0, width, height),
        }
    }

    /// Adds a dirty rectangle to the damage region, clipping it to screen bounds
    /// and merging overlapping regions.
    pub fn add_rect(&mut self, rect: Rect) {
        let Some(clipped) = rect.intersection(&self.screen_bounds) else {
            return;
        };

        if clipped.is_empty() {
            return;
        }

        // 1. Update overall damage bounding box
        self.bounds = match self.bounds {
            Some(existing) => Some(existing.union(&clipped)),
            None => Some(clipped),
        };

        // 2. Try merging with an existing overlapping rectangle
        let mut merged = false;
        for existing in self.rects.iter_mut() {
            if existing.intersects(&clipped) {
                *existing = existing.union(&clipped);
                merged = true;
                break;
            }
        }

        if !merged {
            if self.rects.len() >= MAX_DIRTY_RECTS {
                // If too many fragmented rects, collapse into single bounding box
                if let Some(b) = self.bounds {
                    self.rects.clear();
                    self.rects.push(b);
                }
            } else {
                self.rects.push(clipped);
            }
        }
    }

    /// Marks the entire screen as dirty (e.g. wallpaper change or mode switch).
    pub fn add_fullscreen(&mut self) {
        self.rects.clear();
        self.rects.push(self.screen_bounds);
        self.bounds = Some(self.screen_bounds);
    }

    /// Returns `true` if any portion of the screen is damaged.
    pub fn is_dirty(&self) -> bool {
        self.bounds.is_some()
    }

    /// Clears all dirty regions once flushed to physical VRAM.
    pub fn clear(&mut self) {
        self.rects.clear();
        self.bounds = None;
    }

    /// Returns the minimal enclosing rectangle covering all dirty regions.
    pub fn bounding_box(&self) -> Option<Rect> {
        self.bounds
    }

    /// Returns a slice of individual dirty rectangles.
    pub fn rects(&self) -> &[Rect] {
        &self.rects
    }

    /// Computes total dirty pixel count across all dirty regions.
    pub fn total_dirty_pixels(&self) -> u64 {
        self.rects
            .iter()
            .map(|r| (r.width as u64) * (r.height as u64))
            .sum()
    }
}
