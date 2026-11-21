//! Integrate OS — GPU 2D Acceleration Command Set
//!
//! Encapsulates asynchronous 2D graphics rendering commands and ring buffer queues.

use alloc::collections::VecDeque;
use super::surface::SurfaceId;

/// Hardware / software 2D acceleration rendering primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCommand {
    /// No operation.
    Nop,
    /// Clears an entire surface with a uniform 32-bit color.
    Clear {
        surface: SurfaceId,
        color: u32,
    },
    /// Fills an axis-aligned rectangular region with a solid 32-bit color.
    FillRect {
        surface: SurfaceId,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: u32,
    },
    /// Performs a high-speed block image transfer (bitblt) between surfaces.
    Blit {
        src: SurfaceId,
        dst: SurfaceId,
        src_x: i32,
        src_y: i32,
        dst_x: i32,
        dst_y: i32,
        width: u32,
        height: u32,
    },
    /// Draws a straight line using Bresenham's algorithm.
    DrawLine {
        surface: SurfaceId,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: u32,
    },
    /// Swaps or presents the backbuffer surface to the front scanout display.
    Present {
        surface: SurfaceId,
    },
    /// Inserts a synchronization barrier returning a sequence fence.
    Fence {
        fence_id: u32,
    },
}

/// FIFO command ring buffer for GPU acceleration requests.
pub struct GpuCommandQueue {
    queue: VecDeque<GpuCommand>,
    capacity: usize,
}

impl GpuCommandQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, cmd: GpuCommand) -> Result<(), &'static str> {
        if self.queue.len() >= self.capacity {
            return Err("GPU Command Queue is full");
        }
        self.queue.push_back(cmd);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<GpuCommand> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
