//! Integrate OS — Desktop Wallpaper & Branding Watermark
//!
//! Renders the desktop background vertical gradient and the minimalist
//! geometric Integrate OS isometric emblem watermark.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::wm::theme::{DESKTOP_BG_BOTTOM, DESKTOP_BG_TOP};

/// Watermark line color (subtle translucent slate/cyan).
pub const WATERMARK_COLOR: u32 = 0xFF243647;
pub const WATERMARK_ACCENT: u32 = 0xFF2C4358;

/// Renders the complete desktop background with branding watermark onto `canvas`.
pub fn render_wallpaper(canvas: &mut Canvas) {
    let screen_rect = Rect::new(0, 0, canvas.width, canvas.height);
    let top_color = Color32::from_u32(DESKTOP_BG_TOP);
    let bottom_color = Color32::from_u32(DESKTOP_BG_BOTTOM);

    // 1. Vertical linear gradient
    canvas.draw_gradient_v(screen_rect, top_color, bottom_color);

    // 2. Geometric Isometric Cube Emblem (Integrate OS Watermark)
    // Positioned in the center-right region
    let cx = ((canvas.width as i32) * 65) / 100;
    let cy = (canvas.height as i32) / 2;
    let s = 90; // scale factor
    let dx = (s * 87) / 100; // cos(30 deg) * s approx 0.866 * s
    let dy = s / 2;          // sin(30 deg) * s = 0.5 * s

    let col_main = Color32::from_u32(WATERMARK_COLOR);
    let col_acc = Color32::from_u32(WATERMARK_ACCENT);

    // Outer hexagon vertices:
    // v0: top (cx, cy - s)
    // v1: top-right (cx + dx, cy - dy)
    // v2: bottom-right (cx + dx, cy + dy)
    // v3: bottom (cx, cy + s)
    // v4: bottom-left (cx - dx, cy + dy)
    // v5: top-left (cx - dx, cy - dy)
    // v_center: center (cx, cy)
    let v0 = (cx, cy - s);
    let v1 = (cx + dx, cy - dy);
    let v2 = (cx + dx, cy + dy);
    let v3 = (cx, cy + s);
    let v4 = (cx - dx, cy + dy);
    let v5 = (cx - dx, cy - dy);
    let vc = (cx, cy);

    // Outer silhouette
    canvas.draw_line(v0.0, v0.1, v1.0, v1.1, col_main);
    canvas.draw_line(v1.0, v1.1, v2.0, v2.1, col_main);
    canvas.draw_line(v2.0, v2.1, v3.0, v3.1, col_main);
    canvas.draw_line(v3.0, v3.1, v4.0, v4.1, col_main);
    canvas.draw_line(v4.0, v4.1, v5.0, v5.1, col_main);
    canvas.draw_line(v5.0, v5.1, v0.0, v0.1, col_main);

    // Inner isometric Y-spokes
    canvas.draw_line(vc.0, vc.1, v0.0, v0.1, col_acc);
    canvas.draw_line(vc.0, vc.1, v2.0, v2.1, col_acc);
    canvas.draw_line(vc.0, vc.1, v4.0, v4.1, col_acc);

    // Concentric inner nested cube
    let s_in = (s * 55) / 100;
    let dx_in = (s_in * 87) / 100;
    let dy_in = s_in / 2;

    let in0 = (cx, cy - s_in);
    let in1 = (cx + dx_in, cy - dy_in);
    let in2 = (cx + dx_in, cy + dy_in);
    let in3 = (cx, cy + s_in);
    let in4 = (cx - dx_in, cy + dy_in);
    let in5 = (cx - dx_in, cy - dy_in);

    canvas.draw_line(in0.0, in0.1, in1.0, in1.1, col_main);
    canvas.draw_line(in1.0, in1.1, in2.0, in2.1, col_main);
    canvas.draw_line(in2.0, in2.1, in3.0, in3.1, col_main);
    canvas.draw_line(in3.0, in3.1, in4.0, in4.1, col_main);
    canvas.draw_line(in4.0, in4.1, in5.0, in5.1, col_main);
    canvas.draw_line(in5.0, in5.1, in0.0, in0.1, col_main);

    canvas.draw_line(vc.0, vc.1, in0.0, in0.1, col_acc);
    canvas.draw_line(vc.0, vc.1, in2.0, in2.1, col_acc);
    canvas.draw_line(vc.0, vc.1, in4.0, in4.1, col_acc);
}
