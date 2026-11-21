//! Integrate OS — GPU Driver Trait & Abstraction
//!
//! Provides the central hardware abstraction interface implemented by all GPU drivers.

use super::command::GpuCommand;
use super::device::GpuDevice;
use super::surface::{GpuSurface, SurfaceId};
use super::types::*;

/// Universal driver interface implemented by physical and software GPU drivers.
pub trait GpuDriver: Send + Sync {
    /// Returns the human-readable driver name.
    fn name(&self) -> &'static str;

    /// Returns the GPU vendor classification.
    fn vendor(&self) -> GpuVendor;

    /// Returns the underlying PCI display device if present.
    fn pci_device(&self) -> Option<&GpuDevice>;

    /// Initializes hardware registers, sets default mode, and allocates scanout.
    fn init(&mut self) -> Result<(), GpuError>;

    /// Returns the GPU hardware capability flags and memory limits.
    fn capabilities(&self) -> GpuCapabilities;

    /// Returns the list of display modes supported by the hardware.
    fn supported_modes(&self) -> &[DisplayMode];

    /// Returns the current active display resolution and color format.
    fn current_mode(&self) -> DisplayMode;

    /// Switches the display adapter to a new display mode.
    fn set_mode(&mut self, mode: DisplayMode) -> Result<(), GpuError>;

    /// Returns linear framebuffer memory mapping details.
    fn framebuffer_info(&self) -> FramebufferInfo;

    /// Submits a 2D rendering command to the GPU queue.
    fn submit_command(&mut self, cmd: GpuCommand) -> Result<(), GpuError>;

    /// Flushes all queued commands and returns the latest completed fence ID.
    fn flush_commands(&mut self) -> Result<u32, GpuError>;

    /// Blocks until the specified fence sequence has completed execution.
    fn wait_fence(&mut self, fence_id: u32) -> Result<(), GpuError>;

    /// Allocates an offscreen render target surface.
    fn create_surface(
        &mut self,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<SurfaceId, GpuError>;

    /// Destroys and deallocates an offscreen surface.
    fn destroy_surface(&mut self, id: SurfaceId) -> Result<(), GpuError>;

    /// Retrieves an immutable reference to an allocated surface.
    fn get_surface(&self, id: SurfaceId) -> Option<&GpuSurface>;

    /// Retrieves a mutable reference to an allocated surface.
    fn get_surface_mut(&mut self, id: SurfaceId) -> Option<&mut GpuSurface>;
}
