//! Window Manager Drawing & Compositing Routines
//!
//! Renders desktop wallpaper, window frames, title bars, control buttons,
//! and blits offscreen client surface buffers onto the target display canvas.

use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::FONT_HEIGHT;
use crate::graphics::rect::Rect;
use crate::wm::theme::*;
use crate::wm::types::*;
use crate::wm::window::Window;

/// Renders the full desktop background.
pub fn render_desktop(canvas: &mut Canvas) {
    let screen = Rect::new(0, 0, canvas.width, canvas.height);
    canvas.draw_gradient_v(
        screen,
        Color32::from_u32(DESKTOP_BG_TOP),
        Color32::from_u32(DESKTOP_BG_BOTTOM),
    );
}

/// Renders a managed window including decorations, title bar, controls, and client buffer.
pub fn render_window(canvas: &mut Canvas, window: &Window) {
    if window.state == WindowState::Minimized || window.state == WindowState::Hidden {
        return;
    }

    // 1. Render Window Decorations (if not borderless)
    if !window.flags.contains(WindowFlags::BORDERLESS) {
        let border_color = if window.active {
            Color32::from_u32(BORDER_COLOR_ACTIVE)
        } else {
            Color32::from_u32(BORDER_COLOR_INACTIVE)
        };

        // Draw outer 2px border
        canvas.draw_rect(window.bounds, border_color);
        canvas.draw_rect(window.bounds.inset(1, 1), border_color);

        // Draw title bar
        let tb = window.title_bar_rect();
        if window.active {
            canvas.draw_gradient_v(
                tb,
                Color32::from_u32(TITLE_BAR_ACTIVE_TOP),
                Color32::from_u32(TITLE_BAR_ACTIVE_BOTTOM),
            );
        } else {
            canvas.draw_filled_rect(tb, Color32::from_u32(TITLE_BAR_INACTIVE));
        }

        // Draw title text
        let tx = tb.x + 8;
        let ty = tb.y + ((TITLE_BAR_HEIGHT.saturating_sub(FONT_HEIGHT as u32)) / 2) as i32;
        let text_fg = if window.active {
            Color32::from_u32(TITLE_TEXT_ACTIVE)
        } else {
            Color32::from_u32(TITLE_TEXT_INACTIVE)
        };
        canvas.draw_text(tx, ty, &window.title, text_fg, None);

        // Draw control buttons
        if window.flags.contains(WindowFlags::CLOSABLE) {
            let cr = window.close_button_rect();
            canvas.draw_filled_rect(cr, Color32::from_u32(BTN_CLOSE));
            canvas.draw_rect(cr, Color32::from_u32(0xFF882222));
        }
        if window.flags.contains(WindowFlags::MAXIMIZABLE) {
            let mr = window.maximize_button_rect();
            canvas.draw_filled_rect(mr, Color32::from_u32(BTN_MAXIMIZE));
            canvas.draw_rect(mr, Color32::from_u32(0xFF226622));
        }
        if window.flags.contains(WindowFlags::MINIMIZABLE) {
            let nr = window.minimize_button_rect();
            canvas.draw_filled_rect(nr, Color32::from_u32(BTN_MINIMIZE));
            canvas.draw_rect(nr, Color32::from_u32(0xFF885500));
        }
    }

    // 2. Blit Client Content Buffer into Client Rectangle with Strict Clipping
    let cr = window.client_rect();
    let prev_clip = canvas.clip_rect;
    canvas.set_clip_rect(cr);

    let (cw, ch) = window.client_size();
    for cy in 0..ch {
        let sy = cr.y + cy as i32;
        for cx in 0..cw {
            let sx = cr.x + cx as i32;
            let idx = (cy as usize) * (cw as usize) + (cx as usize);
            if idx < window.content_buffer.len() {
                let color_raw = window.content_buffer[idx];
                canvas.draw_pixel(sx, sy, Color32::from_u32(color_raw));
            }
        }
    }

    canvas.set_clip_rect(prev_clip);
}
