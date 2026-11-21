//! Visual Compositing Effects: Alpha Blending & Drop Shadows
//!
//! Provides software alpha compositing, opacity scaling, and multi-layered
//! window drop shadows.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;

/// Standard Porter-Duff Source-Over alpha blend of two 32-bit ARGB pixels.
#[inline(always)]
pub fn blend_argb(fg: u32, bg: u32) -> u32 {
    let a = (fg >> 24) & 0xFF;
    if a == 255 {
        return fg;
    }
    if a == 0 {
        return bg;
    }

    let inv_a = 255 - a;

    let fg_r = (fg >> 16) & 0xFF;
    let fg_g = (fg >> 8) & 0xFF;
    let fg_b = fg & 0xFF;

    let bg_r = (bg >> 16) & 0xFF;
    let bg_g = (bg >> 8) & 0xFF;
    let bg_b = bg & 0xFF;

    let out_r = (fg_r * a + bg_r * inv_a) / 255;
    let out_g = (fg_g * a + bg_g * inv_a) / 255;
    let out_b = (fg_b * a + bg_b * inv_a) / 255;

    0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b
}

/// Scales the alpha component of a 32-bit ARGB color by an opacity factor (0..255).
#[inline(always)]
pub fn apply_opacity(color: u32, opacity: u8) -> u32 {
    let a = ((color >> 24) & 0xFF) as u32;
    let new_a = (a * (opacity as u32)) / 255;
    (color & 0x00FF_FFFF) | ((new_a & 0xFF) << 24)
}

/// Renders a soft multi-layered drop shadow around a window rectangle.
pub fn draw_drop_shadow(canvas: &mut Canvas, rect: Rect, shadow_size: u32, active: bool) {
    if rect.is_empty() || shadow_size == 0 {
        return;
    }

    let base_alpha: u32 = if active { 40 } else { 20 };
    // Offset shadow slightly down and right
    let shadow_offset_x = 2;
    let shadow_offset_y = 3;

    for i in 1..=shadow_size {
        let alpha = (base_alpha / (i + 1)).max(2) as u8;
        let shadow_color = Color32::rgba(0, 0, 0, alpha);

        // Expand rectangle outward for each layer
        let x = rect.x + shadow_offset_x - i as i32;
        let y = rect.y + shadow_offset_y - i as i32;
        let w = rect.width + (i * 2);
        let h = rect.height + (i * 2);

        let shadow_rect = Rect::new(x, y, w, h);

        // Draw shadow perimeter lines using alpha blending
        if let Some(clipped) = shadow_rect.intersection(&canvas.clip_rect) {
            // Top and bottom spans
            for px in clipped.x..clipped.right() {
                if !rect.contains(px, shadow_rect.y) {
                    canvas.draw_pixel_blend(px, shadow_rect.y, shadow_color);
                }
                if !rect.contains(px, shadow_rect.bottom() - 1) {
                    canvas.draw_pixel_blend(px, shadow_rect.bottom() - 1, shadow_color);
                }
            }
            // Left and right spans
            for py in clipped.y..clipped.bottom() {
                if !rect.contains(shadow_rect.x, py) {
                    canvas.draw_pixel_blend(shadow_rect.x, py, shadow_color);
                }
                if !rect.contains(shadow_rect.right() - 1, py) {
                    canvas.draw_pixel_blend(shadow_rect.right() - 1, py, shadow_color);
                }
            }
        }
    }
}
