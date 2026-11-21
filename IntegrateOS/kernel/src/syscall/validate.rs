//! User Pointer & Buffer Security Validation
//!
//! Enforces strict bounds, canonical address layout, and MMU permission verification
//! on all memory pointers passed from User Space across the syscall boundary.

use crate::memory::paging::{PageTableFlags, PAGE_MANAGER};
use crate::syscall::error::SyscallError;

/// Minimum allowable user virtual address (protects low 64 KiB against null pointer dereference).
pub const USER_SPACE_MIN: u64 = 0x0000_0000_0001_0000;

/// Upper limit of canonical 64-bit lower-half user address space.
pub const USER_SPACE_MAX: u64 = 0x0000_7FFF_FFFF_FFFF;

/// Validates that a user-provided memory buffer lies entirely within mapped, authorized user space.
pub fn validate_user_pointer(
    ptr: u64,
    len: usize,
    require_writable: bool,
) -> Result<(), SyscallError> {
    if len == 0 {
        return Ok(());
    }

    // 1. Lower boundary check (rejects NULL and low BIOS/IVT memory)
    if ptr < USER_SPACE_MIN {
        return Err(SyscallError::BadAddress);
    }

    // 2. Arithmetic overflow check
    let end = ptr
        .checked_add(len as u64)
        .ok_or(SyscallError::BadAddress)?;

    // 3. Upper boundary check (rejects non-canonical and higher-half supervisor addresses)
    if end > USER_SPACE_MAX {
        return Err(SyscallError::BadAddress);
    }

    // 4. Page table permission validation for every 4 KiB page spanned by the buffer
    let page_start = ptr & !0xFFF;
    let page_end = (end + 0xFFF) & !0xFFF;
    let page_mgr_lock = PAGE_MANAGER.lock();
    let Some(page_mgr) = page_mgr_lock.as_ref() else {
        return Err(SyscallError::BadAddress);
    };

    let mut current_page = page_start;
    while current_page < page_end {
        let Some(flags) = page_mgr.get_page_flags(current_page) else {
            return Err(SyscallError::BadAddress); // Unmapped virtual page
        };

        if !flags.contains(PageTableFlags::PRESENT) {
            return Err(SyscallError::BadAddress);
        }

        if !flags.contains(PageTableFlags::USER_ACCESSIBLE) {
            return Err(SyscallError::BadAddress); // Supervisor-only page
        }

        if require_writable && !flags.contains(PageTableFlags::WRITABLE) {
            return Err(SyscallError::BadAddress); // Read-only page
        }

        current_page += 4096;
    }

    Ok(())
}

/// Validates and constructs an immutable byte slice from a user memory buffer.
pub fn validate_user_slice<'a>(ptr: u64, len: usize) -> Result<&'a [u8], SyscallError> {
    validate_user_pointer(ptr, len, false)?;
    // SAFETY: Bounds and page table permissions verified above.
    Ok(unsafe { core::slice::from_raw_parts(ptr as *const u8, len) })
}

/// Validates and constructs a mutable byte slice from a user memory buffer.
pub fn validate_user_slice_mut<'a>(ptr: u64, len: usize) -> Result<&'a mut [u8], SyscallError> {
    validate_user_pointer(ptr, len, true)?;
    // SAFETY: Bounds and page table permissions verified above.
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) })
}
