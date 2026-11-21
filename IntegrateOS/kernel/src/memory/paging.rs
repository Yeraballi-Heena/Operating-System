//! x86_64 4-Level Paging & Virtual Memory Management
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Implements hardware 4-level paging structures for x86_64:
//! - Level 4: Page Map Level 4 (PML4)
//! - Level 3: Page Directory Pointer Table (PDPT)
//! - Level 2: Page Directory (PD)
//! - Level 1: Page Table (PT)
//!
//! Provides:
//! - Page table entry flags (Present, Writable, UserAccessible, WriteThrough, NoCache, Accessed, Dirty, HugePage, Global, NoExecute)
//! - Virtual-to-physical address translation
//! - Page mapping (`map_page`) using frames from `FRAME_ALLOCATOR`
//! - Page unmapping (`unmap_page`)
//! - TLB single-page invalidation (`invlpg`)

use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::sync::IrqSpinlock;

/// Standard 4-level paging entry flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableFlags(pub u64);

impl PageTableFlags {
    pub const NONE: Self = Self(0);
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER_ACCESSIBLE: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const NO_CACHE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for PageTableFlags {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

/// A 64-bit page table entry (PTE).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    #[inline(always)]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn is_present(&self) -> bool {
        (self.0 & PageTableFlags::PRESENT.0) != 0
    }

    #[inline(always)]
    pub const fn is_huge(&self) -> bool {
        (self.0 & PageTableFlags::HUGE_PAGE.0) != 0
    }

    #[inline(always)]
    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags(self.0 & !Self::ADDR_MASK)
    }

    #[inline(always)]
    pub const fn physical_address(&self) -> u64 {
        self.0 & Self::ADDR_MASK
    }

    #[inline(always)]
    pub fn set(&mut self, phys_addr: u64, flags: PageTableFlags) {
        self.0 = (phys_addr & Self::ADDR_MASK) | flags.0;
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

/// A 4 KiB page table consisting of 512 entries.
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }

    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

/// Invalidates the TLB entry for a single virtual address via `invlpg`.
#[inline(always)]
pub fn flush_tlb(virt_addr: u64) {
    // SAFETY: Invalidating TLB entry for specified linear address.
    unsafe {
        core::arch::asm!(
            "invlpg [{}]",
            in(reg) virt_addr,
            options(nostack, preserves_flags)
        );
    }
}

/// Decomposes a 64-bit canonical virtual address into its 9-bit 4-level paging indices.
#[derive(Debug, Clone, Copy)]
pub struct PageIndices {
    pub pml4_idx: usize,
    pub pdpt_idx: usize,
    pub pd_idx: usize,
    pub pt_idx: usize,
    pub offset: usize,
}

impl PageIndices {
    pub const fn from_virtual(addr: u64) -> Self {
        Self {
            pml4_idx: ((addr >> 39) & 0x1FF) as usize,
            pdpt_idx: ((addr >> 30) & 0x1FF) as usize,
            pd_idx: ((addr >> 21) & 0x1FF) as usize,
            pt_idx: ((addr >> 12) & 0x1FF) as usize,
            offset: (addr & 0xFFF) as usize,
        }
    }
}

/// Core virtual memory manager wrapping the active PML4 table.
pub struct PageManager {
    pml4_phys: u64,
}

impl PageManager {
    /// Creates a PageManager pointing to the current active PML4 table.
    pub const fn new(pml4_phys: u64) -> Self {
        Self { pml4_phys }
    }

    /// Returns a mutable reference to a physical page table.
    /// In early boot (first 4 GiB identity mapped), physical address == virtual address.
    #[inline(always)]
    unsafe fn get_table_mut(&self, phys_addr: u64) -> &'static mut PageTable {
        // SAFETY: The first 4 GiB is identity mapped in early boot.
        unsafe { &mut *(phys_addr as *mut PageTable) }
    }

    /// Traverses the active page tables to translate a virtual address to its physical address.
    pub fn translate(&self, virt_addr: u64) -> Option<u64> {
        let indices = PageIndices::from_virtual(virt_addr);

        // SAFETY: Traversing identity-mapped page table structures.
        unsafe {
            let pml4 = self.get_table_mut(self.pml4_phys);
            let pml4e = &pml4.entries[indices.pml4_idx];
            if !pml4e.is_present() {
                return None;
            }

            let pdpt = self.get_table_mut(pml4e.physical_address());
            let pdpte = &pdpt.entries[indices.pdpt_idx];
            if !pdpte.is_present() {
                return None;
            }
            if pdpte.is_huge() {
                // 1 GiB huge page
                return Some(pdpte.physical_address() + (virt_addr & 0x3FFF_FFFF));
            }

            let pd = self.get_table_mut(pdpte.physical_address());
            let pde = &pd.entries[indices.pd_idx];
            if !pde.is_present() {
                return None;
            }
            if pde.is_huge() {
                // 2 MiB huge page
                return Some(pde.physical_address() + (virt_addr & 0x1F_FFFF));
            }

            let pt = self.get_table_mut(pde.physical_address());
            let pte = &pt.entries[indices.pt_idx];
            if !pte.is_present() {
                return None;
            }

            Some(pte.physical_address() + (indices.offset as u64))
        }
    }

    /// Returns the effective page table entry flags for a virtual address.
    pub fn get_page_flags(&self, virt_addr: u64) -> Option<PageTableFlags> {
        let indices = PageIndices::from_virtual(virt_addr);
        // SAFETY: Traversing identity-mapped page table structures.
        unsafe {
            let pml4 = self.get_table_mut(self.pml4_phys);
            let pml4e = &pml4.entries[indices.pml4_idx];
            if !pml4e.is_present() {
                return None;
            }

            let pdpt = self.get_table_mut(pml4e.physical_address());
            let pdpte = &pdpt.entries[indices.pdpt_idx];
            if !pdpte.is_present() {
                return None;
            }
            if pdpte.is_huge() {
                return Some(pdpte.flags());
            }

            let pd = self.get_table_mut(pdpte.physical_address());
            let pde = &pd.entries[indices.pd_idx];
            if !pde.is_present() {
                return None;
            }
            if pde.is_huge() {
                return Some(pde.flags());
            }

            let pt = self.get_table_mut(pde.physical_address());
            let pte = &pt.entries[indices.pt_idx];
            if !pte.is_present() {
                return None;
            }

            Some(pte.flags())
        }
    }

    /// Maps a 4 KiB virtual page to a physical frame with specified flags.
    /// Allocates intermediate page tables from `FRAME_ALLOCATOR` as needed.
    pub fn map_page(
        &mut self,
        virt_addr: u64,
        phys_addr: u64,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let indices = PageIndices::from_virtual(virt_addr);
        let is_user = flags.contains(PageTableFlags::USER_ACCESSIBLE);
        let table_flags = if is_user {
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        } else {
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE
        };

        // SAFETY: Identity mapped page tables traversal and allocation.
        unsafe {
            let pml4 = self.get_table_mut(self.pml4_phys);

            // Level 4 -> Level 3
            if !pml4.entries[indices.pml4_idx].is_present() {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame()
                    .ok_or("Out of memory for PDPT")?;
                let table = self.get_table_mut(frame);
                table.zero();
                pml4.entries[indices.pml4_idx].set(frame, table_flags);
            } else if is_user {
                let existing_flags = pml4.entries[indices.pml4_idx].flags();
                pml4.entries[indices.pml4_idx].set(
                    pml4.entries[indices.pml4_idx].physical_address(),
                    existing_flags | PageTableFlags::USER_ACCESSIBLE,
                );
            }
            let pdpt_phys = pml4.entries[indices.pml4_idx].physical_address();
            let pdpt = self.get_table_mut(pdpt_phys);

            // Level 3 -> Level 2
            if !pdpt.entries[indices.pdpt_idx].is_present() {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame()
                    .ok_or("Out of memory for PD")?;
                let table = self.get_table_mut(frame);
                table.zero();
                pdpt.entries[indices.pdpt_idx].set(frame, table_flags);
            } else if pdpt.entries[indices.pdpt_idx].is_huge() {
                return Err("Cannot map within existing 1 GiB huge page");
            } else if is_user {
                let existing_flags = pdpt.entries[indices.pdpt_idx].flags();
                pdpt.entries[indices.pdpt_idx].set(
                    pdpt.entries[indices.pdpt_idx].physical_address(),
                    existing_flags | PageTableFlags::USER_ACCESSIBLE,
                );
            }
            let pd_phys = pdpt.entries[indices.pdpt_idx].physical_address();
            let pd = self.get_table_mut(pd_phys);

            // Level 2 -> Level 1
            if !pd.entries[indices.pd_idx].is_present() {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame()
                    .ok_or("Out of memory for PT")?;
                let table = self.get_table_mut(frame);
                table.zero();
                pd.entries[indices.pd_idx].set(frame, table_flags);
            } else if pd.entries[indices.pd_idx].is_huge() {
                // If it's a 2 MiB identity huge page from bootstrap, split it into 4 KiB entries
                self.split_huge_page_2mb(&mut pd.entries[indices.pd_idx])?;
            }
            if is_user {
                let existing_flags = pd.entries[indices.pd_idx].flags();
                pd.entries[indices.pd_idx].set(
                    pd.entries[indices.pd_idx].physical_address(),
                    existing_flags | PageTableFlags::USER_ACCESSIBLE,
                );
            }
            let pt_phys = pd.entries[indices.pd_idx].physical_address();
            let pt = self.get_table_mut(pt_phys);

            // Level 1: Map 4 KiB page
            let entry = &mut pt.entries[indices.pt_idx];
            entry.set(phys_addr, flags | PageTableFlags::PRESENT);
            flush_tlb(virt_addr);
            Ok(())
        }
    }

    /// Splits a 2 MiB huge page into 512 standard 4 KiB page entries.
    unsafe fn split_huge_page_2mb(
        &mut self,
        pde: &mut PageTableEntry,
    ) -> Result<(), &'static str> {
        let huge_phys = pde.physical_address();
        let huge_flags = pde.flags();

        let pt_frame = FRAME_ALLOCATOR
            .lock()
            .allocate_frame()
            .ok_or("Out of memory to split 2 MiB page")?;

        // SAFETY: Zeroing and populating new 4 KiB page table entries.
        unsafe {
            let pt = self.get_table_mut(pt_frame);
            let sub_flags = (huge_flags.0 & !PageTableFlags::HUGE_PAGE.0) | PageTableFlags::PRESENT.0;
            for i in 0..512 {
                let page_phys = huge_phys + (i * PAGE_SIZE) as u64;
                pt.entries[i].set(page_phys, PageTableFlags(sub_flags));
            }

            // Update PDE to point to the new PT table (not huge anymore)
            pde.set(pt_frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }

        Ok(())
    }

    /// Unmaps a 4 KiB virtual page, clearing its entry and invalidating the TLB.
    pub fn unmap_page(&mut self, virt_addr: u64) -> Result<u64, &'static str> {
        let indices = PageIndices::from_virtual(virt_addr);

        // SAFETY: Traversing page tables to clear PTE.
        unsafe {
            let pml4 = self.get_table_mut(self.pml4_phys);
            let pml4e = &pml4.entries[indices.pml4_idx];
            if !pml4e.is_present() {
                return Err("PML4 entry not present");
            }

            let pdpt = self.get_table_mut(pml4e.physical_address());
            let pdpte = &pdpt.entries[indices.pdpt_idx];
            if !pdpte.is_present() || pdpte.is_huge() {
                return Err("PDPT entry not present or is huge");
            }

            let pd = self.get_table_mut(pdpte.physical_address());
            let pde = &pd.entries[indices.pd_idx];
            if !pde.is_present() || pde.is_huge() {
                return Err("PD entry not present or is huge");
            }

            let pt = self.get_table_mut(pde.physical_address());
            let pte = &mut pt.entries[indices.pt_idx];
            if !pte.is_present() {
                return Err("Page table entry not present");
            }

            let phys_addr = pte.physical_address();
            pte.clear();
            flush_tlb(virt_addr);
            Ok(phys_addr)
        }
    }
}

pub static PAGE_MANAGER: IrqSpinlock<Option<PageManager>> = IrqSpinlock::new(None);

/// Initializes the system PageManager using the active PML4 physical address.
pub fn init(pml4_phys: u64) {
    let manager = PageManager::new(pml4_phys);
    *PAGE_MANAGER.lock() = Some(manager);
}
