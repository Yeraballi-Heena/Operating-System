//! System Call Dispatcher & Implementation Routines
//!
//! Validates arguments and dispatches system calls from User Space (Ring 3)
//! to the appropriate kernel services.

use crate::arch::x86_64::keyboard::KEYBOARD_QUEUE;
use crate::arch::x86_64::pit;
use crate::process::manager::PROCESS_MANAGER;
use crate::syscall::error::SyscallError;
use crate::syscall::numbers::*;
use crate::syscall::validate::{validate_user_slice, validate_user_slice_mut};
use crate::kprint;

/// Primary kernel system call dispatcher.
pub fn dispatch_syscall(
    num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> i64 {
    match num {
        SYS_EXIT => sys_exit(arg1 as i32),
        SYS_WRITE => sys_write(arg1, arg2, arg3 as usize),
        SYS_READ => sys_read(arg1, arg2, arg3 as usize),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_YIELD => sys_yield(),
        SYS_SLEEP => sys_sleep(arg1),
        SYS_UPTIME => sys_uptime(),
        SYS_TEST_ECHO => sys_test_echo(arg1),
        SYS_OPEN => sys_open(arg1, arg2 as usize, arg3 as u32),
        SYS_CLOSE => sys_close(arg1),
        SYS_SEEK => sys_seek(arg1, arg2 as i64, arg3),
        SYS_STAT => sys_stat(arg1, arg2 as usize, arg3),
        SYS_MKDIR => sys_mkdir(arg1, arg2 as usize),
        _ => SyscallError::NoSuchSyscall.as_i64(),
    }
}

/// `SYS_EXIT` (1): Terminates the calling process with exit status.
pub fn sys_exit(code: i32) -> i64 {
    let mut pm = PROCESS_MANAGER.lock();
    if let Some(curr) = pm.current() {
        let pid = curr.pid;
        pm.terminate(pid, code);
    }
    code as i64
}

/// `SYS_WRITE` (2): Writes up to `len` bytes from user buffer to file descriptor `fd`.
pub fn sys_write(fd: u64, buf_ptr: u64, len: usize) -> i64 {
    let slice = match validate_user_slice(buf_ptr, len) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    if fd == 1 || fd == 2 {
        if let Ok(s) = core::str::from_utf8(slice) {
            kprint!("{}", s);
        } else {
            for &b in slice {
                kprint!("{}", b as char);
            }
        }
        return len as i64;
    }

    match crate::fs::write(fd as usize, slice) {
        Ok(bytes) => bytes as i64,
        Err(_) => SyscallError::BadFileDescriptor.as_i64(),
    }
}

/// `SYS_READ` (3): Reads up to `len` bytes from file descriptor `fd` into user buffer.
pub fn sys_read(fd: u64, buf_ptr: u64, len: usize) -> i64 {
    let slice = match validate_user_slice_mut(buf_ptr, len) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    if fd == 0 {
        let mut bytes_read = 0;
        while bytes_read < len {
            if let Some(ch) = KEYBOARD_QUEUE.lock().pop() {
                slice[bytes_read] = ch;
                bytes_read += 1;
            } else {
                break;
            }
        }
        return bytes_read as i64;
    }

    match crate::fs::read(fd as usize, slice) {
        Ok(bytes) => bytes as i64,
        Err(_) => SyscallError::BadFileDescriptor.as_i64(),
    }
}

/// `SYS_GETPID` (4): Returns the PID of the calling process.
pub fn sys_getpid() -> i64 {
    PROCESS_MANAGER
        .lock()
        .current()
        .map(|p| p.pid.as_u64() as i64)
        .unwrap_or(0)
}

/// `SYS_GETPPID` (5): Returns the parent PID of the calling process.
pub fn sys_getppid() -> i64 {
    PROCESS_MANAGER
        .lock()
        .current()
        .map(|p| p.ppid.as_u64() as i64)
        .unwrap_or(0)
}

/// `SYS_YIELD` (6): Voluntarily yields the remainder of the process's CPU timeslice.
pub fn sys_yield() -> i64 {
    crate::scheduler::yield_now();
    0
}

/// `SYS_SLEEP` (7): Suspends execution for at least `ticks` timer ticks (100 Hz).
pub fn sys_sleep(ticks: u64) -> i64 {
    crate::scheduler::sleep_current(ticks);
    0
}

/// `SYS_UPTIME` (8): Returns the total monotonic timer ticks since kernel boot.
pub fn sys_uptime() -> i64 {
    pit::get_ticks() as i64
}

/// `SYS_TEST_ECHO` (10): Verifies argument passing integrity by returning `val * 2`.
pub fn sys_test_echo(val: u64) -> i64 {
    (val * 2) as i64
}

/// `SYS_OPEN` (11): Opens a file at `path` with `flags`.
pub fn sys_open(path_ptr: u64, path_len: usize, flags: u32) -> i64 {
    let slice = match validate_user_slice(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    let path_str = match core::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidArgument.as_i64(),
    };

    match crate::fs::open(path_str, crate::fs::OpenFlags(flags)) {
        Ok(fd) => fd as i64,
        Err(_) => SyscallError::NoSuchFileOrDirectory.as_i64(),
    }
}

/// `SYS_CLOSE` (12): Closes an open file descriptor.
pub fn sys_close(fd: u64) -> i64 {
    match crate::fs::close(fd as usize) {
        Ok(()) => 0,
        Err(_) => SyscallError::BadFileDescriptor.as_i64(),
    }
}

/// `SYS_SEEK` (13): Repositions file offset.
pub fn sys_seek(fd: u64, offset: i64, whence: u64) -> i64 {
    let seek_from = match whence {
        0 => crate::fs::SeekFrom::Start(offset as u64),
        1 => crate::fs::SeekFrom::Current(offset),
        2 => crate::fs::SeekFrom::End(offset),
        _ => return SyscallError::InvalidArgument.as_i64(),
    };

    match crate::fs::seek(fd as usize, seek_from) {
        Ok(new_off) => new_off as i64,
        Err(_) => SyscallError::BadFileDescriptor.as_i64(),
    }
}

/// `SYS_STAT` (14): Retrieves metadata for a file path into a user Stat buffer.
pub fn sys_stat(path_ptr: u64, path_len: usize, stat_buf_ptr: u64) -> i64 {
    let path_slice = match validate_user_slice(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidArgument.as_i64(),
    };

    let meta = match crate::fs::stat(path_str) {
        Ok(m) => m,
        Err(_) => return SyscallError::NoSuchFileOrDirectory.as_i64(),
    };

    let out_slice = match validate_user_slice_mut(stat_buf_ptr, core::mem::size_of::<UserStat>()) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    let user_stat = UserStat {
        size: meta.size,
        node_type: meta.node_type as u32,
        inode: meta.inode,
        mode: meta.mode,
    };

    // SAFETY: out_slice is validated for size of UserStat and writability.
    unsafe {
        let dest = out_slice.as_mut_ptr() as *mut UserStat;
        core::ptr::write_unaligned(dest, user_stat);
    }

    0
}

/// User stat structure representation for sys_stat.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserStat {
    pub size: u64,
    pub node_type: u32,
    pub inode: u64,
    pub mode: u32,
}

/// `SYS_MKDIR` (15): Creates a directory at `path`.
pub fn sys_mkdir(path_ptr: u64, path_len: usize) -> i64 {
    let slice = match validate_user_slice(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e.as_i64(),
    };

    let path_str = match core::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidArgument.as_i64(),
    };

    match crate::fs::mkdir(path_str) {
        Ok(()) => 0,
        Err(_) => SyscallError::NoSuchFileOrDirectory.as_i64(),
    }
}
