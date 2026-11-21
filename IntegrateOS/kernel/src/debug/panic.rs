//! Enhanced Panic Diagnostics & Stack Frame Unwinding
//!
//! Provides bare-metal frame-pointer stack walking, register snapshots, and
//! memory hex dumping for post-mortem kernel failure analysis.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// Reads the current value of the base pointer (`RBP`) register.
#[inline(always)]
pub fn read_rbp() -> u64 {
    let rbp: u64;
    // SAFETY: Reading current RBP register.
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack));
    }
    rbp
}

/// Reads the current value of the stack pointer (`RSP`) register.
#[inline(always)]
pub fn read_rsp() -> u64 {
    let rsp: u64;
    // SAFETY: Reading current RSP register.
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack));
    }
    rsp
}

/// An unwound stack frame descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackFrame {
    pub depth: usize,
    pub rbp: u64,
    pub rip: u64,
}

/// Unwinds the current stack frame pointer chain (`RBP`), capturing return instruction pointers.
pub fn unwind_stack() -> Vec<StackFrame> {
    let mut frames = Vec::new();
    let mut curr_rbp = read_rbp();
    let curr_rsp = read_rsp();
    let mut depth = 0;

    // 1. Primary path: Traverse the RBP linked list up to a safe maximum depth of 32 frames
    while curr_rbp != 0 && (curr_rbp & 7) == 0 && depth < 32 {
        if curr_rbp < 0x10000 {
            break;
        }

        // SAFETY: Pointer is aligned and above low reserved memory.
        let saved_rbp = unsafe { core::ptr::read_volatile(curr_rbp as *const u64) };
        let return_rip = unsafe { core::ptr::read_volatile((curr_rbp + 8) as *const u64) };

        frames.push(StackFrame {
            depth,
            rbp: curr_rbp,
            rip: return_rip,
        });

        // On x86_64, the stack grows downwards; caller frame pointers must have higher addresses
        if saved_rbp <= curr_rbp || saved_rbp == 0 {
            break;
        }

        curr_rbp = saved_rbp;
        depth += 1;
    }

    // 2. Fallback path: If RBP chain produced no frames, scan stack starting at RSP for return RIPs
    if frames.is_empty() && curr_rsp >= 0x10000 && (curr_rsp & 7) == 0 {
        let mut scan_ptr = curr_rsp;
        let scan_limit = curr_rsp + 2048; // Scan up to 2 KiB above RSP
        while scan_ptr < scan_limit && depth < 32 {
            // SAFETY: Memory is within active kernel stack.
            let val = unsafe { core::ptr::read_volatile(scan_ptr as *const u64) };
            // Check if val is a plausible kernel text return address (1 MiB to 16 MiB range)
            if val >= 0x0010_0000 && val < 0x0100_0000 {
                frames.push(StackFrame {
                    depth,
                    rbp: scan_ptr,
                    rip: val,
                });
                depth += 1;
            }
            scan_ptr += 8;
        }
    }

    frames
}

/// Formats a contiguous memory buffer as a canonical hex dump with ASCII representation.
pub fn format_hex_dump(addr: u64, len: usize) -> String {
    let mut output = String::new();
    let rows = (len + 15) / 16;

    for row in 0..rows {
        let row_addr = addr + (row * 16) as u64;
        let _ = write!(output, "{:#018x}: ", row_addr);

        let row_len = if (row + 1) * 16 <= len {
            16
        } else {
            len % 16
        };

        // Hex bytes
        for col in 0..16 {
            if col < row_len {
                // SAFETY: Reading validated length of bytes.
                let byte = unsafe { core::ptr::read_volatile((row_addr + col as u64) as *const u8) };
                let _ = write!(output, "{:02x} ", byte);
            } else {
                let _ = write!(output, "   ");
            }
            if col == 7 {
                let _ = write!(output, " ");
            }
        }

        let _ = write!(output, " |");
        // ASCII representation
        for col in 0..row_len {
            let byte = unsafe { core::ptr::read_volatile((row_addr + col as u64) as *const u8) };
            if byte >= 32 && byte <= 126 {
                output.push(byte as char);
            } else {
                output.push('.');
            }
        }
        output.push_str("|\n");
    }

    output
}
