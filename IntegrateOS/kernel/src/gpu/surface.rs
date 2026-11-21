//! Integrate OS — GPU Surface Management & Offscreen Render Targets
//!
//! Provides pixel surface buffers, offscreen render targets, and scanout memory management.

use alloc::vec;
use alloc::vec::Vec;
use super::types::*;

/// Strongly typed identifier for an allocated GPU surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId(pub u32);

impl SurfaceId {
    pub const SCANOUT: Self = Self(0);
}

/// An allocated 2D pixel buffer residing in system memory or video memory.
pub struct GpuSurface {
    pub id: SurfaceId,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub format: PixelFormat,
    pub buffer: Vec<u8>,
    pub is_scanout: bool,
}

impl GpuSurface {
    /// Creates a new offscreen render target surface.
    pub fn new(id: SurfaceId, width: u32, height: u32, format: PixelFormat) -> Self {
        let bpp = format.bytes_per_pixel() as u32;
        let pitch = width * bpp;
        let total_size = (pitch * height) as usize;
        let buffer = vec![0u8; total_size];

        Self {
            id,
            width,
            height,
            pitch,
            format,
            buffer,
            is_scanout: false,
        }
    }

    /// Creates the primary display scanout surface.
    pub fn new_scanout(width: u32, height: u32, pitch: u32, format: PixelFormat) -> Self {
        let total_size = (pitch * height) as usize;
        let buffer = vec![0u8; total_size];

        Self {
            id: SurfaceId::SCANOUT,
            width,
            height,
            pitch,
            format,
            buffer,
            is_scanout: true,
        }
    }

    pub fn size_bytes(&self) -> usize {
        self.buffer.len()
    }

    /// Clears the entire surface to a single 32-bit color.
    pub fn clear(&mut self, color: u32) {
        let bpp = self.format.bytes_per_pixel() as usize;
        if bpp == 4 {
            let bytes = color.to_le_bytes();
            for chunk in self.buffer.chunks_exact_mut(4) {
                chunk.copy_from_slice(&bytes);
            }
        } else {
            self.buffer.fill(0);
        }
    }

    /// Sets a single pixel color with coordinate bounds check.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let bpp = self.format.bytes_per_pixel() as usize;
        let offset = (y as usize * self.pitch as usize) + (x as usize * bpp);
        if offset + 4 <= self.buffer.len() {
            self.buffer[offset..offset + 4].copy_from_slice(&color.to_le_bytes());
        }
    }

    /// Reads a single pixel color with coordinate bounds check.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let bpp = self.format.bytes_per_pixel() as usize;
        let offset = (y as usize * self.pitch as usize) + (x as usize * bpp);
        if offset + 4 <= self.buffer.len() {
            u32::from_le_bytes([
                self.buffer[offset],
                self.buffer[offset + 1],
                self.buffer[offset + 2],
                self.buffer[offset + 3],
            ])
        } else {
            0
        }
    }

    /// Fills an axis-aligned rectangle with a solid 32-bit color.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x + w as i32).max(0) as u32).min(self.width);
        let y1 = ((y + h as i32).max(0) as u32).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let bpp = self.format.bytes_per_pixel() as usize;
        let color_bytes = color.to_le_bytes();

        for py in y0..y1 {
            let row_start = (py as usize * self.pitch as usize) + (x0 as usize * bpp);
            let row_end = (py as usize * self.pitch as usize) + (x1 as usize * bpp);
            for offset in (row_start..row_end).step_by(bpp) {
                if offset + 4 <= self.buffer.len() {
                    self.buffer[offset..offset + 4].copy_from_slice(&color_bytes);
                }
            }
        }
    }
}

/// Tracks and manages all allocated GPU surfaces.
pub struct SurfaceManager {
    surfaces: Vec<GpuSurface>,
    next_id: u32,
}

impl SurfaceManager {
    pub fn new() -> Self {
        Self {
            surfaces: Vec::new(),
            next_id: 1, // 0 is reserved for SCANOUT
        }
    }

    /// Registers the primary scanout surface.
    pub fn register_scanout(&mut self, scanout: GpuSurface) {
        self.surfaces.retain(|s| s.id != SurfaceId::SCANOUT);
        self.surfaces.push(scanout);
    }

    /// Allocates a new offscreen render target surface.
    pub fn create_surface(
        &mut self,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<SurfaceId, GpuError> {
        let id = SurfaceId(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or(GpuError::AllocationFailed)?;

        let surface = GpuSurface::new(id, width, height, format);
        self.surfaces.push(surface);
        Ok(id)
    }

    /// Finds a surface by ID immutably.
    pub fn get_surface(&self, id: SurfaceId) -> Option<&GpuSurface> {
        self.surfaces.iter().find(|s| s.id == id)
    }

    /// Finds a surface by ID mutably.
    pub fn get_surface_mut(&mut self, id: SurfaceId) -> Option<&mut GpuSurface> {
        self.surfaces.iter_mut().find(|s| s.id == id)
    }

    /// Destroys and deallocates an offscreen surface.
    pub fn destroy_surface(&mut self, id: SurfaceId) -> Result<(), GpuError> {
        if id == SurfaceId::SCANOUT {
            return Err(GpuError::UnsupportedOperation);
        }
        let pos = self
            .surfaces
            .iter()
            .position(|s| s.id == id)
            .ok_or(GpuError::InvalidSurface)?;
        self.surfaces.remove(pos);
        Ok(())
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }
}
