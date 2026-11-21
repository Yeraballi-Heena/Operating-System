//! Integrate OS — Oracle VirtualBox Graphics Adapter Driver (VBE/BGA)
//!
//! Controls hardware mode setting, linear framebuffer apertures, and acceleration
//! for the VirtualBox PCI Display Adapter (`0x80EE:0xBEEF`).

use crate::graphics::bga::{is_bga_present, set_video_mode};
use super::super::command::GpuCommand;
use super::super::device::GpuDevice;
use super::super::engine::GpuEngine;
use super::super::surface::{GpuSurface, SurfaceId};
use super::super::traits::GpuDriver;
use super::super::types::*;

pub struct VboxGpuDriver {
    pci_device: GpuDevice,
    engine: GpuEngine,
    current_mode: DisplayMode,
    supported_modes: [DisplayMode; 5],
    capabilities: GpuCapabilities,
}

impl VboxGpuDriver {
    pub fn new(pci_dev: GpuDevice) -> Self {
        let vram = pci_dev.vram_size;
        let capabilities = GpuCapabilities::vbox_accelerated(vram);

        Self {
            pci_device: pci_dev,
            engine: GpuEngine::new(),
            current_mode: DisplayMode::MODE_1024X768,
            supported_modes: [
                DisplayMode::MODE_640X480,
                DisplayMode::MODE_800X600,
                DisplayMode::MODE_1024X768,
                DisplayMode::MODE_1280X720,
                DisplayMode::MODE_1920X1080,
            ],
            capabilities,
        }
    }
}

impl GpuDriver for VboxGpuDriver {
    fn name(&self) -> &'static str {
        "VirtualBox PCI Graphics Adapter (VBE/BGA)"
    }

    fn vendor(&self) -> GpuVendor {
        GpuVendor::VirtualBox
    }

    fn pci_device(&self) -> Option<&GpuDevice> {
        Some(&self.pci_device)
    }

    fn init(&mut self) -> Result<(), GpuError> {
        if !is_bga_present() {
            return Err(GpuError::HardwareError);
        }

        self.set_mode(DisplayMode::MODE_1024X768)?;

        // Allocate primary scanout surface matching mode
        let scanout = GpuSurface::new_scanout(
            self.current_mode.width,
            self.current_mode.height,
            self.current_mode.pitch(),
            self.current_mode.format,
        );
        self.engine.surface_mgr.register_scanout(scanout);

        Ok(())
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
        if !is_bga_present() {
            return Err(GpuError::HardwareError);
        }

        let mode_res = set_video_mode(mode.width, mode.height, mode.bpp);
        if mode_res.is_err() {
            return Err(GpuError::ModeNotSupported);
        }

        self.current_mode = mode;

        // Reallocate scanout surface
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
        let size_bytes = self.current_mode.framebuffer_size();

        FramebufferInfo {
            phys_addr,
            virt_addr: phys_addr, // 1:1 identity-mapped kernel lower memory space
            size_bytes,
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
