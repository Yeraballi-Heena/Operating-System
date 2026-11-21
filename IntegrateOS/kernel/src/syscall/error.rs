//! System Call Error Codes (POSIX/Standard ABI Mapping)
//!
//! Provides `SyscallError` and negative integer conversion for user-space return values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum SyscallError {
    Success = 0,
    PermissionDenied = -1,      // EPERM
    NoSuchFileOrDirectory = -2, // ENOENT
    NoSuchProcess = -3,         // ESRCH
    IoError = -5,               // EIO
    BadFileDescriptor = -9,     // EBADF
    OutOfMemory = -12,          // ENOMEM
    BadAddress = -14,           // EFAULT (Invalid / unauthorized pointer)
    InvalidArgument = -22,      // EINVAL
    NoSuchSyscall = -38,        // ENOSYS (Unimplemented system call)
}

impl SyscallError {
    #[inline(always)]
    pub const fn as_i64(self) -> i64 {
        self as i64
    }
}
