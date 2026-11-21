//! Unified Kernel Error Codes & POSIX Errno Taxonomy
//!
//! Provides strongly typed error codes, subsystem category mapping, human-readable
//! descriptions, and standard POSIX errno translation.

/// Subsystem error category for diagnostics and telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelErrorCategory {
    Hardware,
    Memory,
    FileSystem,
    Network,
    Process,
    Security,
    Device,
    Generic,
}

impl KernelErrorCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Hardware => "Hardware",
            Self::Memory => "Memory",
            Self::FileSystem => "FileSystem",
            Self::Network => "Network",
            Self::Process => "Process",
            Self::Security => "Security",
            Self::Device => "Device",
            Self::Generic => "Generic",
        }
    }
}

/// Standardized kernel error code matching standard POSIX errno values where applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum KernelErrorCode {
    Success = 0,
    OperationNotPermitted = 1,          // EPERM
    NoSuchFileOrDirectory = 2,          // ENOENT
    NoSuchProcess = 3,                  // ESRCH
    Interrupted = 4,                    // EINTR
    IoError = 5,                        // EIO
    NoSuchDevice = 6,                   // ENXIO
    BadFileDescriptor = 9,              // EBADF
    ResourceTemporarilyUnavailable = 11,// EAGAIN / EWOULDBLOCK
    OutOfMemory = 12,                   // ENOMEM
    PermissionDenied = 13,              // EACCES
    BadAddress = 14,                    // EFAULT
    DeviceBusy = 16,                    // EBUSY
    FileExists = 17,                    // EEXIST
    NotADirectory = 20,                 // ENOTDIR
    IsADirectory = 21,                  // EISDIR
    InvalidArgument = 22,               // EINVAL
    TooManyOpenFiles = 24,              // EMFILE
    NoSpaceLeftOnDevice = 28,           // ENOSPC
    BrokenPipe = 32,                    // EPIPE
    NotSupported = 95,                  // EOPNOTSUPP
    ConnectionRefused = 111,            // ECONNREFUSED
    TimedOut = 110,                     // ETIMEDOUT
}

impl KernelErrorCode {
    /// Returns the integer POSIX errno representation.
    #[inline(always)]
    pub const fn as_errno(&self) -> i32 {
        *self as i32
    }

    /// Translates an integer errno back to a strongly typed `KernelErrorCode`.
    pub const fn from_errno(errno: i32) -> Option<Self> {
        match errno {
            0 => Some(Self::Success),
            1 => Some(Self::OperationNotPermitted),
            2 => Some(Self::NoSuchFileOrDirectory),
            3 => Some(Self::NoSuchProcess),
            4 => Some(Self::Interrupted),
            5 => Some(Self::IoError),
            6 => Some(Self::NoSuchDevice),
            9 => Some(Self::BadFileDescriptor),
            11 => Some(Self::ResourceTemporarilyUnavailable),
            12 => Some(Self::OutOfMemory),
            13 => Some(Self::PermissionDenied),
            14 => Some(Self::BadAddress),
            16 => Some(Self::DeviceBusy),
            17 => Some(Self::FileExists),
            20 => Some(Self::NotADirectory),
            21 => Some(Self::IsADirectory),
            22 => Some(Self::InvalidArgument),
            24 => Some(Self::TooManyOpenFiles),
            28 => Some(Self::NoSpaceLeftOnDevice),
            32 => Some(Self::BrokenPipe),
            95 => Some(Self::NotSupported),
            110 => Some(Self::TimedOut),
            111 => Some(Self::ConnectionRefused),
            _ => None,
        }
    }

    /// Returns the diagnostic subsystem category for this error.
    pub const fn category(&self) -> KernelErrorCategory {
        match self {
            Self::Success => KernelErrorCategory::Generic,
            Self::OperationNotPermitted | Self::PermissionDenied => KernelErrorCategory::Security,
            Self::NoSuchFileOrDirectory
            | Self::BadFileDescriptor
            | Self::FileExists
            | Self::NotADirectory
            | Self::IsADirectory
            | Self::NoSpaceLeftOnDevice => KernelErrorCategory::FileSystem,
            Self::NoSuchProcess | Self::Interrupted | Self::TooManyOpenFiles => {
                KernelErrorCategory::Process
            }
            Self::IoError | Self::NoSuchDevice | Self::DeviceBusy => KernelErrorCategory::Device,
            Self::OutOfMemory | Self::BadAddress => KernelErrorCategory::Memory,
            Self::BrokenPipe | Self::ConnectionRefused | Self::TimedOut => {
                KernelErrorCategory::Network
            }
            Self::ResourceTemporarilyUnavailable
            | Self::InvalidArgument
            | Self::NotSupported => KernelErrorCategory::Generic,
        }
    }

    /// Returns a human-readable diagnostic message explaining the error.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Success => "Operation completed successfully",
            Self::OperationNotPermitted => "Operation not permitted",
            Self::NoSuchFileOrDirectory => "No such file or directory",
            Self::NoSuchProcess => "No such process",
            Self::Interrupted => "Interrupted system call",
            Self::IoError => "Input/output hardware error",
            Self::NoSuchDevice => "No such device or address",
            Self::BadFileDescriptor => "Bad file descriptor",
            Self::ResourceTemporarilyUnavailable => "Resource temporarily unavailable",
            Self::OutOfMemory => "Cannot allocate memory",
            Self::PermissionDenied => "Permission denied",
            Self::BadAddress => "Bad user address or memory fault",
            Self::DeviceBusy => "Device or resource busy",
            Self::FileExists => "File exists",
            Self::NotADirectory => "Not a directory",
            Self::IsADirectory => "Is a directory",
            Self::InvalidArgument => "Invalid argument",
            Self::TooManyOpenFiles => "Too many open files",
            Self::NoSpaceLeftOnDevice => "No space left on device",
            Self::BrokenPipe => "Broken pipe",
            Self::NotSupported => "Operation not supported",
            Self::ConnectionRefused => "Connection refused",
            Self::TimedOut => "Connection or operation timed out",
        }
    }
}
