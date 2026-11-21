//! User Memory Pointer & Buffer Security Validation
//!
//! Enforces hardware-backed bounds, canonical address layout, alignment, and
//! MMU page table permission validation on all memory pointers originating from User Space (Ring 3).

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::memory::paging::{PageTableFlags, PAGE_MANAGER};
use crate::security::types::PointerViolation;

/// Minimum allowable user virtual address (low 64 KiB protected against NULL / IVT / BDA dereferencing).
pub const USER_SPACE_MIN: u64 = 0x0000_0000_0001_0000;

/// Upper limit of canonical 64-bit lower-half user address space (128 TiB boundary).
pub const USER_SPACE_MAX: u64 = 0x0000_7FFF_FFFF_FFFF;

/// Start of higher-half canonical supervisor address space.
pub const KERNEL_SPACE_MIN: u64 = 0xFFFF_8000_0000_0000;

/// Validates that a user-provided memory region lies entirely within mapped, authorized user space.
pub fn validate_user_address(
    addr: u64,
    len: usize,
    require_writable: bool,
) -> Result<(), PointerViolation> {
    // 1. Check for NULL pointer
    if addr == 0 {
        return Err(PointerViolation::NullPointer);
    }

    // 2. Reject low reserved memory (< 64 KiB)
    if addr < USER_SPACE_MIN {
        return Err(PointerViolation::UnderflowLowMemory);
    }

    // 3. Reject higher-half supervisor addresses and non-canonical holes
    if addr > USER_SPACE_MAX {
        return Err(PointerViolation::KernelAddressViolation);
    }

    if len == 0 {
        return Ok(());
    }

    // 4. Arithmetic overflow check on address + length
    let end = addr
        .checked_add(len as u64)
        .ok_or(PointerViolation::ArithmeticOverflow)?;

    // 5. Upper boundary check against lower-half limit
    if end > USER_SPACE_MAX {
        return Err(PointerViolation::KernelAddressViolation);
    }

    // 6. Page table permission validation for every 4 KiB page spanned by the buffer
    let page_start = addr & !0xFFF;
    let page_end = (end + 0xFFF) & !0xFFF;

    let page_mgr_lock = PAGE_MANAGER.lock();
    let Some(page_mgr) = page_mgr_lock.as_ref() else {
        return Err(PointerViolation::PageNotPresent { addr });
    };

    let mut current_page = page_start;
    while current_page < page_end {
        let Some(flags) = page_mgr.get_page_flags(current_page) else {
            return Err(PointerViolation::PageNotPresent { addr: current_page });
        };

        if !flags.contains(PageTableFlags::PRESENT) {
            return Err(PointerViolation::PageNotPresent { addr: current_page });
        }

        if !flags.contains(PageTableFlags::USER_ACCESSIBLE) {
            return Err(PointerViolation::SupervisorPage { addr: current_page });
        }

        if require_writable && !flags.contains(PageTableFlags::WRITABLE) {
            return Err(PointerViolation::WriteOnReadOnly { addr: current_page });
        }

        current_page += 4096;
    }

    Ok(())
}

/// Strongly typed user-space pointer wrapper with alignment and MMU bounds verification.
#[derive(Debug, Clone, Copy)]
pub struct UserPtr<T> {
    addr: u64,
    _marker: PhantomData<*const T>,
}

impl<T> UserPtr<T> {
    /// Validates and constructs a new typed user pointer.
    pub fn new(addr: u64) -> Result<Self, PointerViolation> {
        if addr == 0 {
            return Err(PointerViolation::NullPointer);
        }

        let align = core::mem::align_of::<T>();
        if addr as usize % align != 0 {
            return Err(PointerViolation::Unaligned {
                required: align,
                actual: addr as usize % align,
            });
        }

        let size = core::mem::size_of::<T>();
        validate_user_address(addr, size, false)?;

        Ok(Self {
            addr,
            _marker: PhantomData,
        })
    }

    /// Returns the raw virtual address.
    #[inline(always)]
    pub fn addr(&self) -> u64 {
        self.addr
    }

    /// Reads a copy of value `T` from user memory after verifying read permissions.
    pub fn read(&self) -> Result<T, PointerViolation>
    where
        T: Copy,
    {
        let size = core::mem::size_of::<T>();
        validate_user_address(self.addr, size, false)?;
        // SAFETY: Address, size, alignment, and page table permissions verified above.
        unsafe {
            Ok(core::ptr::read_volatile(self.addr as *const T))
        }
    }

    /// Writes value `T` into user memory after verifying write permissions.
    pub fn write(&self, val: T) -> Result<(), PointerViolation>
    where
        T: Copy,
    {
        let size = core::mem::size_of::<T>();
        validate_user_address(self.addr, size, true)?;
        // SAFETY: Address, size, alignment, and writable page table permissions verified above.
        unsafe {
            core::ptr::write_volatile(self.addr as *mut T, val);
        }
        Ok(())
    }
}

/// Strongly typed contiguous user-space slice wrapper.
#[derive(Debug, Clone, Copy)]
pub struct UserSlice<T> {
    addr: u64,
    count: usize,
    _marker: PhantomData<T>,
}

impl<T> UserSlice<T> {
    /// Validates and constructs a user memory slice.
    pub fn new(addr: u64, count: usize) -> Result<Self, PointerViolation> {
        let size = core::mem::size_of::<T>();
        let byte_len = count
            .checked_mul(size)
            .ok_or(PointerViolation::ArithmeticOverflow)?;

        let align = core::mem::align_of::<T>();
        if addr as usize % align != 0 {
            return Err(PointerViolation::Unaligned {
                required: align,
                actual: addr as usize % align,
            });
        }

        validate_user_address(addr, byte_len, false)?;

        Ok(Self {
            addr,
            count,
            _marker: PhantomData,
        })
    }

    /// Number of elements in the user slice.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Copies data from user space into a kernel slice buffer.
    pub fn copy_to_slice(&self, dest: &mut [T]) -> Result<(), PointerViolation>
    where
        T: Copy,
    {
        if dest.len() < self.count {
            return Err(PointerViolation::ArithmeticOverflow);
        }
        let size = core::mem::size_of::<T>();
        let byte_len = self.count * size;
        validate_user_address(self.addr, byte_len, false)?;

        // SAFETY: Permissions and bounds verified.
        unsafe {
            core::ptr::copy_nonoverlapping(self.addr as *const T, dest.as_mut_ptr(), self.count);
        }
        Ok(())
    }

    /// Copies data from kernel buffer into user space memory.
    pub fn copy_from_slice(&self, src: &[T]) -> Result<(), PointerViolation>
    where
        T: Copy,
    {
        if src.len() > self.count {
            return Err(PointerViolation::ArithmeticOverflow);
        }
        let size = core::mem::size_of::<T>();
        let byte_len = src.len() * size;
        validate_user_address(self.addr, byte_len, true)?;

        // SAFETY: Writable permissions and bounds verified.
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), self.addr as *mut T, src.len());
        }
        Ok(())
    }
}

/// Safely copies a null-terminated UTF-8 string from user memory up to `max_len` bytes.
pub fn copy_str_from_user(ptr: u64, max_len: usize) -> Result<String, PointerViolation> {
    if max_len == 0 {
        return Ok(String::new());
    }

    let mut bytes = Vec::new();
    let mut current_ptr = ptr;

    for _ in 0..max_len {
        validate_user_address(current_ptr, 1, false)?;
        // SAFETY: Byte validated.
        let b = unsafe { *(current_ptr as *const u8) };
        if b == 0 {
            break;
        }
        bytes.push(b);
        current_ptr += 1;
    }

    String::from_utf8(bytes).map_err(|_| PointerViolation::ZeroLength)
}
