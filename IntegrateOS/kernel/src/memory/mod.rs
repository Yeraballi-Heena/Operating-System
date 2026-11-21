//! Integrate OS — Memory Management Layer
//!
//! - `mmap`: BIOS E820 / Multiboot memory map parser
//! - `frame_allocator`: Bitmap-backed 4 KiB physical page frame allocator
//! - `paging`: 4-level x86_64 paging, dynamic virtual memory mapping, translation & TLB flush
//! - `heap`: Kernel heap allocator & Rust `GlobalAlloc` implementation

pub mod frame_allocator;
pub mod heap;
pub mod mmap;
pub mod paging;
