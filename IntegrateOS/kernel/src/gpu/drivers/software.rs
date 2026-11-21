//! Integrate OS — Universal Software Fallback GPU Driver
//!
//! Provides a software-driven GPU implementation with full 2D surface rendering,
//! offscreen targets, and framebuffer presentation.

use super::super::command::GpuCommand;
use super::super::device::GpuDevice;
use super::super::engine::GpuEngine;
use super::super::surface::{GpuSurface, SurfaceId};
use super::super::traits::GpuDriver;
use super::super::types::*;

pub struct SoftwareGpuDriver {
    engine: GpuEngine,
    current_mode: DisplayMode,
    supported_modes: [DisplayMode; 3],
    capabilities: GpuCapabilities,
}

impl SoftwareGpuDriver {
    pub fn new() -> Self {
        Self {
            engine: GpuEngine::new(),
            current_mode: DisplayMode::MODE_1024X768,
            supported_modes: [
                DisplayMode::MODE_640X480,
                DisplayMode::MODE_800X600,
                DisplayMode::MODE_1024X768,
            ],
            capabilities: GpuCapabilities::software_baseline(),
        }
    }
}

impl GpuDriver for SoftwareGpuDriver {
    fn name(&self) -> &'static str {
        "Integrate OS Software GPU Rasterizer"
    }

    fn vendor(&self) -> GpuVendor {
        GpuVendor::Unknown(0)
    }

    fn pci_device(&self) -> Option<&GpuDevice> {
        None
    }

    fn init(&mut self) -> Result<(), GpuError> {
        self.set_mode(DisplayMode::MODE_1024X768)
    }

    fn capabilities(&self) -> GpuCapabilities {
        self.capabilities
    }

    fn supported_modes(&self) -> &[DisplayMode] {
        &self.supported_modes
    }

    fn current_mode(&self) -> DisplayMode {
        self.current_mode
    }

    fn set_mode(&mut self, mode: DisplayMode) -> Result<(), GpuError> {
        self.current_mode = mode;
        let scanout = GpuSurface::new_scanout(
            mode.width,
            mode.height,
            mode.pitch(),
            mode.format,
        );
        self.engine.surface_mgr.register_scanout(scanout);
        Ok(())
    }

    fn framebuffer_info(&self) -> FramebufferInfo {
        FramebufferInfo {
            phys_addr: crate::graphics::bga::DEFAULT_BGA_LFB_PHYS_ADDR,
            virt_addr: crate::graphics::bga::DEFAULT_BGA_LFB_PHYS_ADDR,
            size_bytes: self.current_mode.framebuffer_size(),
            width: self.current_mode.width,
            height: self.current_mode.height,
            pitch: self.current_mode.pitch(),
            bpp: self.current_mode.bpp,
            format: self.current_mode.format,
        }
    }

    fn submit_command(&mut self, cmd: GpuCommand) -> Result<(), GpuError> {
        self.engine.submit(cmd)
    }

    fn flush_commands(&mut self) -> Result<u32, GpuError> {
        self.engine.flush()
    }

    fn wait_fence(&mut self, fence_id: u32) -> Result<(), GpuError> {
        self.flush_commands()?;
        if self.engine.last_completed_fence >= fence_id {
            Ok(())
        } else {
            Err(GpuError::Timeout)
        }
    }

    fn create_surface(
        &mut self,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<SurfaceId, GpuError> {
        self.engine.surface_mgr.create_surface(width, height, format)
    }

    fn destroy_surface(&mut self, id: SurfaceId) -> Result<(), GpuError> {
        self.engine.surface_mgr.destroy_surface(id)
    }

    fn get_surface(&self, id: SurfaceId) -> Option<&GpuSurface> {
        self.engine.surface_mgr.get_surface(id)
    }

    fn get_surface_mut(&mut self, id: SurfaceId) -> Option<&mut GpuSurface> {
        self.engine.surface_mgr.get_surface_mut(id)
    }
}
