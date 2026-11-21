//! System Call Numbers Specification
//!
//! Standard system call numbers for the Integrate OS System Call ABI.

pub const SYS_EXIT: u64 = 1;
pub const SYS_WRITE: u64 = 2;
pub const SYS_READ: u64 = 3;
pub const SYS_GETPID: u64 = 4;
pub const SYS_GETPPID: u64 = 5;
pub const SYS_YIELD: u64 = 6;
pub const SYS_SLEEP: u64 = 7;
pub const SYS_UPTIME: u64 = 8;
pub const SYS_MMAP: u64 = 9;
pub const SYS_TEST_ECHO: u64 = 10;
pub const SYS_OPEN: u64 = 11;
pub const SYS_CLOSE: u64 = 12;
pub const SYS_SEEK: u64 = 13;
pub const SYS_STAT: u64 = 14;
pub const SYS_MKDIR: u64 = 15;
