//! Integrate OS — System Calls Architecture (Milestone D)
//!
//! Provides:
//! - Standard system call numbers specification (`numbers.rs`)
//! - Strongly typed error codes and POSIX error mappings (`error.rs`)
//! - User memory pointer and buffer security validation (`validate.rs`)
//! - Central system call dispatcher and kernel service routines (`dispatcher.rs`)
//! - Hardware fast system call (`syscall`/`sysret`) MSR initialization (`fast_syscall.rs`)
//! - Milestone D verification test suite (`tests.rs`)

pub mod dispatcher;
pub mod error;
pub mod fast_syscall;
pub mod numbers;
pub mod tests;
pub mod validate;

use crate::klog_ok;

/// Initializes the system call subsystem and hardware fast syscall MSRs.
pub fn init() {
    fast_syscall::init_fast_syscall();
    klog_ok!("System Call Subsystem Initialized: MSRs configured (SCE, STAR, FMASK)");
}
