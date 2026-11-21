//! Integrate OS — GUI Layout System
//!
//! Provides automated layout calculation engines including horizontal and
//! vertical box layouts with padding, item spacing, and alignment.

use alloc::vec::Vec;
use crate::graphics::rect::Rect;
use crate::gui::types::{Alignment, Orientation};

/// Linear Box Layout engine for stacking widgets horizontally or vertically.
#[derive(Debug, Clone, Copy)]
pub struct BoxLayout {
    pub orientation: Orientation,
    pub spacing: u32,
    pub padding: u32,
    pub alignment: Alignment,
}

impl BoxLayout {
    pub const fn new(orientation: Orientation) -> Self {
        Self {
            orientation,
            spacing: 6,
            padding: 8,
            alignment: Alignment::Start,
        }
    }

    pub const fn with_spacing(mut self, spacing: u32) -> Self {
        self.spacing = spacing;
        self
    }

    pub const fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    pub const fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Computes child bounding rectangles given container bounds and item dimensions.
    pub fn compute_bounds(
        &self,
        container_bounds: Rect,
        item_sizes: &[(u32, u32)],
    ) -> Vec<Rect> {
        let mut results = Vec::with_capacity(item_sizes.len());
        if item_sizes.is_empty() {
            return results;
        }

        let pad = self.padding as i32;
        let avail_w = container_bounds.width.saturating_sub(self.padding * 2);
        let avail_h = container_bounds.height.saturating_sub(self.padding * 2);

        match self.orientation {
            Orientation::Vertical => {
                let mut cur_y = container_bounds.y + pad;
                for &(w, h) in item_sizes {
                    let final_w = match self.alignment {
                        Alignment::Stretch => avail_w,
                        _ => w.min(avail_w),
                    };

                    let final_x = match self.alignment {
                        Alignment::Start | Alignment::Stretch => container_bounds.x + pad,
                        Alignment::Center => {
                            let offset = (avail_w.saturating_sub(final_w)) / 2;
                            container_bounds.x + pad + (offset as i32)
                        }
                        Alignment::End => {
                            let offset = avail_w.saturating_sub(final_w);
                            container_bounds.x + pad + (offset as i32)
                        }
                    };

                    results.push(Rect::new(final_x, cur_y, final_w, h));
                    cur_y += (h + self.spacing) as i32;
                }
            }
            Orientation::Horizontal => {
                let mut cur_x = container_bounds.x + pad;
                for &(w, h) in item_sizes {
                    let final_h = match self.alignment {
                        Alignment::Stretch => avail_h,
                        _ => h.min(avail_h),
                    };

                    let final_y = match self.alignment {
                        Alignment::Start | Alignment::Stretch => container_bounds.y + pad,
                        Alignment::Center => {
                            let offset = (avail_h.saturating_sub(final_h)) / 2;
                            container_bounds.y + pad + (offset as i32)
                        }
                        Alignment::End => {
                            let offset = avail_h.saturating_sub(final_h);
                            container_bounds.y + pad + (offset as i32)
                        }
                    };

                    results.push(Rect::new(cur_x, final_y, w, final_h));
                    cur_x += (w + self.spacing) as i32;
                }
            }
        }

        results
    }
}
