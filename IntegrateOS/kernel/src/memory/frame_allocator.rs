//! 4 KiB Physical Page Frame Allocator
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Physical RAM is managed as fixed-size 4 KiB frames using a compact bitmap.
//! - Bit = 0: Frame is free (available for allocation)
//! - Bit = 1: Frame is used/reserved
//!
//! Automatically reserves:
//! - First 1 MiB (BIOS, IVT, BDA, VGA buffer)
//! - Kernel binary (.text, .rodata, .data, .bss, page tables, stack)
//! - Multiboot data structures
//! - Non-usable firmware ranges

use crate::memory::mmap::PhysicalMemorySummary;
use crate::sync::IrqSpinlock;

pub const PAGE_SIZE: usize = 4096;

/// Maximum physical memory tracked by this allocator: 1 GiB (262,144 frames).
/// A 1 GiB bitmap requires 262,144 bits = 32,768 bytes (32 KiB) of static storage.
pub const MAX_TRACKED_FRAMES: usize = 262_144;
pub const BITMAP_WORDS: usize = MAX_TRACKED_FRAMES / 64; // 4096 u64 words

pub struct PhysicalFrameAllocator {
    bitmap: [u64; BITMAP_WORDS],
    total_frames: usize,
    used_frames: usize,
    search_start: usize,
}

impl PhysicalFrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [!0u64; BITMAP_WORDS], // All frames marked used initially
            total_frames: 0,
            used_frames: 0,
            search_start: 0,
        }
    }

    /// Initializes the bitmap based on the firmware memory map summary,
    /// marking usable RAM frames as free and reserving the kernel image.
    pub fn init(&mut self, summary: &PhysicalMemorySummary, kernel_start: u64, kernel_end: u64) {
        // Step 1: Mark everything as used by default
        self.bitmap.fill(!0u64);

        let max_address = summary.highest_usable_address.min((MAX_TRACKED_FRAMES * PAGE_SIZE) as u64);
        self.total_frames = (max_address as usize) / PAGE_SIZE;
        self.used_frames = self.total_frames;

        // Step 2: Unmark (free) all frames that fall within usable E820 regions
        for i in 0..summary.region_count {
            let region = &summary.regions[i];
            if !region.is_usable() {
                continue;
            }

            let start_frame = ((region.base + (PAGE_SIZE as u64) - 1) / (PAGE_SIZE as u64)) as usize;
            let end_frame = ((region.base + region.length) / (PAGE_SIZE as u64)) as usize;
            let end_frame = end_frame.min(self.total_frames);

            for frame in start_frame..end_frame {
                self.set_free(frame);
            }
        }

        // Step 3: Re-reserve low 1 MiB (0x0000_0000 .. 0x0010_0000 = frames 0..256)
        let low_mem_frames = 0x100000 / PAGE_SIZE;
        for frame in 0..low_mem_frames.min(self.total_frames) {
            self.set_used(frame);
        }

        // Step 4: Re-reserve the kernel image range (kernel_start .. kernel_end)
        let k_start_frame = (kernel_start as usize) / PAGE_SIZE;
        let k_end_frame = ((kernel_end as usize) + PAGE_SIZE - 1) / PAGE_SIZE;
        for frame in k_start_frame..k_end_frame.min(self.total_frames) {
            self.set_used(frame);
        }

        self.search_start = k_end_frame;
    }

    #[inline(always)]
    fn set_used(&mut self, frame: usize) {
        if frame < self.total_frames {
            let word_idx = frame / 64;
            let bit_idx = frame % 64;
            if (self.bitmap[word_idx] & (1 << bit_idx)) == 0 {
                self.bitmap[word_idx] |= 1 << bit_idx;
                self.used_frames += 1;
            }
        }
    }

    #[inline(always)]
    fn set_free(&mut self, frame: usize) {
        if frame < self.total_frames {
            let word_idx = frame / 64;
            let bit_idx = frame % 64;
            if (self.bitmap[word_idx] & (1 << bit_idx)) != 0 {
                self.bitmap[word_idx] &= !(1 << bit_idx);
                self.used_frames = self.used_frames.saturating_sub(1);
            }
        }
    }

    /// Allocates a single 4 KiB physical page frame, returning its physical base address.
    pub fn allocate_frame(&mut self) -> Option<u64> {
        let total_words = (self.total_frames + 63) / 64;
        let start_word = self.search_start / 64;

        // Search from search_start to end
        for word_offset in 0..total_words {
            let word_idx = (start_word + word_offset) % total_words;
            let word = self.bitmap[word_idx];
            if word != !0u64 {
                // Found a word with at least one free bit (0)
                let bit_idx = (!word).trailing_zeros() as usize;
                let frame = word_idx * 64 + bit_idx;
                if frame < self.total_frames {
                    self.set_used(frame);
                    self.search_start = frame + 1;
                    return Some((frame * PAGE_SIZE) as u64);
                }
            }
        }

        None
    }

    /// Frees a previously allocated 4 KiB physical page frame by physical address.
    pub fn free_frame(&mut self, phys_addr: u64) {
        let frame = (phys_addr as usize) / PAGE_SIZE;
        if frame < self.total_frames {
            self.set_free(frame);
            if frame < self.search_start {
                self.search_start = frame;
            }
        }
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    pub fn free_frames(&self) -> usize {
        self.total_frames.saturating_sub(self.used_frames)
    }

    pub fn used_frames(&self) -> usize {
        self.used_frames
    }
}

pub static FRAME_ALLOCATOR: IrqSpinlock<PhysicalFrameAllocator> =
    IrqSpinlock::new(PhysicalFrameAllocator::new());
