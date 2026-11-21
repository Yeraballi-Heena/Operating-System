//! Integrate OS — Graphics & Linear Framebuffer Subsystem
//!
//! Provides hardware mode initialization, double-buffered canvas, 2D primitives,
//! boundary clipping, 8x16 bitmap font text rendering, and console integration.

pub mod bga;
pub mod canvas;
pub mod color;
pub mod console;
pub mod demo;
pub mod font;
pub mod rect;

use crate::graphics::bga::{is_bga_present, set_video_mode, BgaModeInfo};
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::console::FramebufferConsole;
use crate::graphics::rect::Rect;
use crate::sync::IrqSpinlock;

pub struct GraphicsContext {
    pub canvas: Canvas,
    pub console: FramebufferConsole,
    pub mode: BgaModeInfo,
}

pub static GRAPHICS: IrqSpinlock<Option<GraphicsContext>> = IrqSpinlock::new(None);

/// Initializes the high-resolution linear framebuffer graphics subsystem.
///
/// Configures the hardware display adapter (BGA) to the requested resolution (e.g. 1024x768x32),
/// allocates the double-buffered back buffer, initializes the text console overlay,
/// and registers the driver in the global state.
pub fn init(width: u32, height: u32, bpp: u8) -> Result<BgaModeInfo, &'static str> {
    if !is_bga_present() {
        return Err("No supported linear framebuffer hardware (BGA) detected");
    }

    let mode_info = set_video_mode(width, height, bpp)?;

    let canvas = Canvas::new(
        mode_info.width,
        mode_info.height,
        mode_info.pitch,
        mode_info.bpp,
        mode_info.phys_addr,
    );

    // Create a 80x20 console pane placed neatly in a desktop terminal window
    let console_bounds = Rect::new(40, 180, 640, 320);
    let console = FramebufferConsole::new(console_bounds, Color32::TEXT_PRIMARY, Color32::PANEL_BG);

    let mut lock = GRAPHICS.lock();
    *lock = Some(GraphicsContext {
        canvas,
        console,
        mode: mode_info,
    });

    Ok(mode_info)
}

/// Executes a closure with mutable access to the active graphics canvas.
pub fn with_canvas<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Canvas) -> R,
{
    let mut lock = GRAPHICS.lock();
    lock.as_mut().map(|ctx| f(&mut ctx.canvas))
}

/// Executes a closure with mutable access to both canvas and console.
pub fn with_graphics<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Canvas, &mut FramebufferConsole) -> R,
{
    let mut lock = GRAPHICS.lock();
    lock.as_mut().map(|ctx| f(&mut ctx.canvas, &mut ctx.console))
}

/// Returns true if the graphical framebuffer is active.
pub fn is_active() -> bool {
    GRAPHICS.lock().is_some()
}
