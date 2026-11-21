//! Kernel Heap & Dynamic Memory Allocator
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Provides a linked-list block allocator managing a dedicated virtual heap range.
//! Connects to Rust's core allocation interface by implementing `GlobalAlloc`.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::paging::{PageTableFlags, PAGE_MANAGER};
use crate::sync::IrqSpinlock;

/// Virtual base address for the kernel heap: higher-half address 0xFFFF_9000_0000_0000.
pub const HEAP_START: u64 = 0xFFFF_9000_0000_0000;
/// Initial heap size mapped on boot: 32 MiB (8192 * 4 KiB pages).
pub const HEAP_INITIAL_SIZE: usize = 32 * 1024 * 1024;
/// Maximum virtual heap limit: 64 MiB.
pub const HEAP_MAX_SIZE: usize = 64 * 1024 * 1024;

/// Header preceding every allocated or free block in the heap.
#[repr(C, align(16))]
struct BlockHeader {
    size: usize,                  // Usable payload size (excluding sizeof(BlockHeader))
    is_free: bool,
    next: Option<NonNull<BlockHeader>>,
}

impl BlockHeader {
    pub const fn new(size: usize) -> Self {
        Self {
            size,
            is_free: true,
            next: None,
        }
    }
}

pub struct LinkedListAllocator {
    head: Option<NonNull<BlockHeader>>,
    heap_start: u64,
    heap_end: u64,
    total_bytes: usize,
    allocated_bytes: usize,
}

unsafe impl Send for LinkedListAllocator {}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: None,
            heap_start: 0,
            heap_end: 0,
            total_bytes: 0,
            allocated_bytes: 0,
        }
    }

    /// Initializes the heap by mapping initial physical frames and setting up the root block.
    pub fn init(&mut self, start: u64, initial_size: usize) -> Result<(), &'static str> {
        self.heap_start = start;
        self.heap_end = start;
        self.total_bytes = 0;
        self.allocated_bytes = 0;

        self.expand_heap(initial_size)?;

        let header_ptr = start as *mut BlockHeader;
        let payload_size = initial_size - core::mem::size_of::<BlockHeader>();
        // SAFETY: The memory range [start .. start + initial_size] was just mapped.
        unsafe {
            core::ptr::write(header_ptr, BlockHeader::new(payload_size));
            self.head = NonNull::new(header_ptr);
        }

        Ok(())
    }

    /// Expands the virtual heap mapping by allocating and mapping additional physical frames.
    fn expand_heap(&mut self, size: usize) -> Result<(), &'static str> {
        let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut page_mgr_lock = PAGE_MANAGER.lock();
        let page_mgr = page_mgr_lock.as_mut().ok_or("PageManager not available")?;

        for _ in 0..pages {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame()
                .ok_or("Out of physical memory to expand heap")?;

            page_mgr.map_page(
                self.heap_end,
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            )?;
            self.heap_end += PAGE_SIZE as u64;
            self.total_bytes += PAGE_SIZE;
        }

        Ok(())
    }

    /// Allocates a block of memory with the specified layout.
    pub fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let _align = layout.align().max(16);
        let size = align_up(layout.size(), 16).max(16);
        let header_size = core::mem::size_of::<BlockHeader>();

        let mut current = self.head;

        while let Some(mut block_ptr) = current {
            // SAFETY: Current pointer is validated non-null and inside heap.
            let block = unsafe { block_ptr.as_mut() };

            if block.is_free && block.size >= size {
                // Check if we can split the block
                let remaining = block.size.saturating_sub(size);
                if remaining >= header_size + 16 {
                    let next_header_addr = (block_ptr.as_ptr() as usize) + header_size + size;
                    let next_header_ptr = next_header_addr as *mut BlockHeader;
                    let next_payload_size = remaining - header_size;

                    // SAFETY: Creating new sub-block header in remaining free space.
                    unsafe {
                        core::ptr::write(
                            next_header_ptr,
                            BlockHeader {
                                size: next_payload_size,
                                is_free: true,
                                next: block.next,
                            },
                        );
                        block.next = NonNull::new(next_header_ptr);
                    }
                    block.size = size;
                }

                block.is_free = false;
                self.allocated_bytes += block.size;

                let payload_ptr = (block_ptr.as_ptr() as usize + header_size) as *mut u8;
                return payload_ptr;
            }

            current = block.next;
        }

        core::ptr::null_mut()
    }

    /// Deallocates a block of memory previously returned by `allocate`.
    pub fn deallocate(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let header_size = core::mem::size_of::<BlockHeader>();
        let block_addr = (ptr as usize) - header_size;
        let block_ptr = block_addr as *mut BlockHeader;

        // SAFETY: Pointer was returned by allocate, so header resides immediately before it.
        unsafe {
            let block = &mut *block_ptr;
            if !block.is_free {
                block.is_free = true;
                self.allocated_bytes = self.allocated_bytes.saturating_sub(block.size);
            }
        }

        // Coalesce adjacent free blocks to prevent fragmentation
        self.coalesce();
    }

    /// Merges consecutive free blocks in the heap.
    fn coalesce(&mut self) {
        let mut current = self.head;
        let header_size = core::mem::size_of::<BlockHeader>();

        while let Some(mut block_ptr) = current {
            // SAFETY: Safe traversal of heap block list.
            let block = unsafe { block_ptr.as_mut() };

            if block.is_free {
                while let Some(mut next_ptr) = block.next {
                    let next_block = unsafe { next_ptr.as_mut() };
                    let expected_next_addr = (block_ptr.as_ptr() as usize) + header_size + block.size;

                    if next_block.is_free && (next_ptr.as_ptr() as usize) == expected_next_addr {
                        block.size += header_size + next_block.size;
                        block.next = next_block.next;
                    } else {
                        break;
                    }
                }
            }

            current = block.next;
        }
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    pub fn free_bytes(&self) -> usize {
        self.total_bytes.saturating_sub(self.allocated_bytes)
    }
}

const fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// Global locked allocator wrapper implementing Rust's `GlobalAlloc` trait.
pub struct LockedHeap(pub IrqSpinlock<LinkedListAllocator>);

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().allocate(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().deallocate(ptr, layout)
    }
}

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap(IrqSpinlock::new(LinkedListAllocator::new()));

/// Initializes the kernel heap and configures Rust's dynamic memory allocation.
pub fn init() -> Result<(), &'static str> {
    ALLOCATOR.0.lock().init(HEAP_START, HEAP_INITIAL_SIZE)
}
