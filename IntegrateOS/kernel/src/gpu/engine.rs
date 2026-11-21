//! Integrate OS — GPU 2D Execution & Rasterization Engine
//!
//! Executes queued 2D primitives, blits, line rasterization, and presentation swaps.

use super::command::{GpuCommand, GpuCommandQueue};
use super::surface::{GpuSurface, SurfaceId, SurfaceManager};
use super::types::*;

/// Software & Hardware accelerated 2D execution pipeline.
pub struct GpuEngine {
    pub surface_mgr: SurfaceManager,
    pub command_queue: GpuCommandQueue,
    pub last_completed_fence: u32,
    pub next_fence_counter: u32,
}

impl GpuEngine {
    pub fn new() -> Self {
        Self {
            surface_mgr: SurfaceManager::new(),
            command_queue: GpuCommandQueue::new(256),
            last_completed_fence: 0,
            next_fence_counter: 1,
        }
    }

    /// Submits a command into the processing queue.
    pub fn submit(&mut self, cmd: GpuCommand) -> Result<(), GpuError> {
        self.command_queue
            .push(cmd)
            .map_err(|_| GpuError::CommandQueueFull)
    }

    /// Executes all pending commands in FIFO order until queue is empty.
    pub fn flush(&mut self) -> Result<u32, GpuError> {
        while let Some(cmd) = self.command_queue.pop() {
            self.execute_command(cmd)?;
        }
        Ok(self.last_completed_fence)
    }

    /// Processes an individual 2D rendering command.
    pub fn execute_command(&mut self, cmd: GpuCommand) -> Result<(), GpuError> {
        match cmd {
            GpuCommand::Nop => Ok(()),

            GpuCommand::Clear { surface, color } => {
                let surf = self
                    .surface_mgr
                    .get_surface_mut(surface)
                    .ok_or(GpuError::InvalidSurface)?;
                surf.clear(color);
                Ok(())
            }

            GpuCommand::FillRect {
                surface,
                x,
                y,
                width,
                height,
                color,
            } => {
                let surf = self
                    .surface_mgr
                    .get_surface_mut(surface)
                    .ok_or(GpuError::InvalidSurface)?;
                surf.fill_rect(x, y, width, height, color);
                Ok(())
            }

            GpuCommand::DrawLine {
                surface,
                x0,
                y0,
                x1,
                y1,
                color,
            } => {
                let surf = self
                    .surface_mgr
                    .get_surface_mut(surface)
                    .ok_or(GpuError::InvalidSurface)?;
                draw_line_bresenham(surf, x0, y0, x1, y1, color);
                Ok(())
            }

            GpuCommand::Blit {
                src,
                dst,
                src_x,
                src_y,
                dst_x,
                dst_y,
                width,
                height,
            } => {
                if src == dst {
                    return Err(GpuError::UnsupportedOperation);
                }
                // Fetch pointers / references safely
                let (src_ptr, src_w, src_h, src_pitch) = {
                    let s = self.surface_mgr.get_surface(src).ok_or(GpuError::InvalidSurface)?;
                    (s.buffer.as_ptr(), s.width, s.height, s.pitch)
                };

                let dst_surf = self
                    .surface_mgr
                    .get_surface_mut(dst)
                    .ok_or(GpuError::InvalidSurface)?;

                blit_surfaces(
                    src_ptr,
                    src_w,
                    src_h,
                    src_pitch,
                    dst_surf,
                    src_x,
                    src_y,
                    dst_x,
                    dst_y,
                    width,
                    height,
                );
                Ok(())
            }

            GpuCommand::Present { surface } => {
                if surface != SurfaceId::SCANOUT {
                    // Copy source surface into SCANOUT
                    let (src_ptr, src_w, src_h, src_pitch) = {
                        let s = self
                            .surface_mgr
                            .get_surface(surface)
                            .ok_or(GpuError::InvalidSurface)?;
                        (s.buffer.as_ptr(), s.width, s.height, s.pitch)
                    };

                    if let Some(scanout) = self.surface_mgr.get_surface_mut(SurfaceId::SCANOUT) {
                        blit_surfaces(
                            src_ptr,
                            src_w,
                            src_h,
                            src_pitch,
                            scanout,
                            0,
                            0,
                            0,
                            0,
                            src_w,
                            src_h,
                        );
                    }
                }

                // If hardware VRAM linear framebuffer is active, present to screen
                crate::graphics::with_canvas(|c| {
                    c.present();
                });
                Ok(())
            }

            GpuCommand::Fence { fence_id } => {
                self.last_completed_fence = fence_id;
                Ok(())
            }
        }
    }
}

/// Bresenham's integer line algorithm.
fn draw_line_bresenham(surf: &mut GpuSurface, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && y0 >= 0 {
            surf.set_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// High-speed block image transfer between surface buffers.
fn blit_surfaces(
    src_ptr: *const u8,
    src_w: u32,
    src_h: u32,
    src_pitch: u32,
    dst: &mut GpuSurface,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
    width: u32,
    height: u32,
) {
    let sx0 = src_x.max(0) as u32;
    let sy0 = src_y.max(0) as u32;
    let dx0 = dst_x.max(0) as u32;
    let dy0 = dst_y.max(0) as u32;

    let copy_w = width.min(src_w.saturating_sub(sx0)).min(dst.width.saturating_sub(dx0));
    let copy_h = height.min(src_h.saturating_sub(sy0)).min(dst.height.saturating_sub(dy0));

    if copy_w == 0 || copy_h == 0 {
        return;
    }

    let bpp = dst.format.bytes_per_pixel() as usize;
    let row_bytes = copy_w as usize * bpp;

    for row in 0..copy_h {
        let sy = sy0 + row;
        let dy = dy0 + row;

        let src_offset = (sy as usize * src_pitch as usize) + (sx0 as usize * bpp);
        let dst_offset = (dy as usize * dst.pitch as usize) + (dx0 as usize * bpp);

        if dst_offset + row_bytes <= dst.buffer.len() {
            // SAFETY: src_ptr points to valid buffer of size src_h * src_pitch, bounds checked above.
            unsafe {
                let src_slice = core::slice::from_raw_parts(src_ptr.add(src_offset), row_bytes);
                dst.buffer[dst_offset..dst_offset + row_bytes].copy_from_slice(src_slice);
            }
        }
    }
}
