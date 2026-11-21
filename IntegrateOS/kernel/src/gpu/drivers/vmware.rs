//! Integrate OS — VMware SVGA II Graphics Driver Stubs
//!
//! Provides register definitions and adapter support for VMware SVGA II (`0x15AD:0x0405`).

use super::super::command::GpuCommand;
use super::super::device::GpuDevice;
use super::super::engine::GpuEngine;
use super::super::surface::{GpuSurface, SurfaceId};
use super::super::traits::GpuDriver;
use super::super::types::*;

pub const SVGA_REG_ID: u32 = 0;
pub const SVGA_REG_ENABLE: u32 = 1;
pub const SVGA_REG_WIDTH: u32 = 2;
pub const SVGA_REG_HEIGHT: u32 = 3;
pub const SVGA_REG_BITS_PER_PIXEL: u32 = 7;
pub const SVGA_REG_FB_OFFSET: u32 = 14;

pub struct VmwareSvgaDriver {
    pci_device: GpuDevice,
    engine: GpuEngine,
    current_mode: DisplayMode,
    capabilities: GpuCapabilities,
}

impl VmwareSvgaDriver {
    pub fn new(pci_dev: GpuDevice) -> Self {
        let vram = pci_dev.vram_size;
        Self {
            pci_device: pci_dev,
            engine: GpuEngine::new(),
            current_mode: DisplayMode::MODE_1024X768,
            capabilities: GpuCapabilities::vbox_accelerated(vram),
        }
    }
}

impl GpuDriver for VmwareSvgaDriver {
    fn name(&self) -> &'static str {
        "VMware SVGA II Driver"
    }

    fn vendor(&self) -> GpuVendor {
        GpuVendor::Vmware
    }

    fn pci_device(&self) -> Option<&GpuDevice> {
        Some(&self.pci_device)
    }

    fn init(&mut self) -> Result<(), GpuError> {
        self.set_mode(DisplayMode::MODE_1024X768)
    }

    fn capabilities(&self) -> GpuCapabilities {
        self.capabilities
    }

    fn supported_modes(&self) -> &[DisplayMode] {
        &[DisplayMode::MODE_1024X768]
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
        let phys_addr = self.pci_device.lfb_phys_address();
        FramebufferInfo {
            phys_addr,
            virt_addr: phys_addr,
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
