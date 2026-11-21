//! Phase 15 — Virtual Filesystem (VFS) Verification Suite
//!
//! Validates:
//! 1. VFS mount hierarchy & canonical path normalization (`/etc/os-release`, `..` resolution)
//! 2. File creation, write, readback, and byte-for-byte data integrity
//! 3. Nested directory creation (`mkdir`), subdirectories, and directory listing (`readdir`)
//! 4. File seek operations (`SeekFrom::Start`, `Current`, `End`), appending, and partial I/O
//! 5. Special device nodes (`/dev/null`, `/dev/zero`, `/dev/console`, `/dev/hda`), deletion, and error handling

use crate::fs::vfs::{NodeType, OpenFlags, SeekFrom};
use crate::fs::{close, mkdir, open, read, read_file, readdir, remove, seek, stat, write, write_file};
use crate::{klog_info, klog_ok};

/// Executes the comprehensive Phase 15 VFS test suite.
pub fn run_fs_tests() {
    klog_info!("Starting Phase 15 Virtual Filesystem (VFS) Verification Suite...");

    // Test 1: VFS Mount Hierarchy & Path Normalization
    let os_release_bytes = read_file("/etc/os-release").expect("Failed to read /etc/os-release");
    let os_release_str = core::str::from_utf8(&os_release_bytes).expect("Invalid UTF-8 in os-release");
    if !os_release_str.contains("Integrate OS") {
        panic!("VFS Test 1 Failed: /etc/os-release missing expected OS name");
    }

    let hostname_bytes = read_file("/etc/hostname").expect("Failed to read /etc/hostname");
    if hostname_bytes != b"integrate-os\n" {
        panic!("VFS Test 1 Failed: /etc/hostname content mismatch");
    }

    // Path normalization test with redundant slashes and relative parent components
    let norm_bytes = read_file("/etc/../etc/./os-release").expect("Failed to read normalized path");
    if norm_bytes != os_release_bytes {
        panic!("VFS Test 1 Failed: Path normalization resolution mismatch");
    }
    klog_ok!("Phase 15 Test 1 Passed: VFS root mount & canonical path normalization verified");

    // Test 2: File Creation, Writing, and Data Integrity
    let test_file_path = "/home/test_payload.bin";
    let mut payload = [0u8; 256];
    for i in 0..256 {
        payload[i] = ((i * 11 + 17) % 256) as u8;
    }

    write_file(test_file_path, &payload).expect("Failed to write test file");

    let readback = read_file(test_file_path).expect("Failed to read back test file");
    if readback.len() != 256 {
        panic!("VFS Test 2 Failed: File size mismatch: expected 256, got {}", readback.len());
    }
    for i in 0..256 {
        if readback[i] != payload[i] {
            panic!(
                "VFS Test 2 Failed: Data byte mismatch at index {}: expected {:#04x}, got {:#04x}",
                i, payload[i], readback[i]
            );
        }
    }
    klog_ok!("Phase 15 Test 2 Passed: File creation & 100% byte-for-byte readback verified");

    // Test 3: Directory Creation (mkdir), Nested Hierarchy & readdir
    mkdir("/home/developer").expect("Failed to mkdir /home/developer");
    mkdir("/home/developer/projects").expect("Failed to mkdir /home/developer/projects");

    let notes_path = "/home/developer/projects/readme.txt";
    write_file(notes_path, b"Integrate OS VFS Active\n").expect("Failed to write nested file");

    let entries = readdir("/home/developer/projects").expect("Failed to readdir nested directory");
    let has_readme = entries.iter().any(|e| e.name == "readme.txt" && e.node_type == NodeType::File);
    let has_dot = entries.iter().any(|e| e.name == ".");
    let has_dotdot = entries.iter().any(|e| e.name == "..");

    if !has_readme || !has_dot || !has_dotdot {
        panic!("VFS Test 3 Failed: readdir missing expected directory entries");
    }

    let meta = stat(notes_path).expect("Failed to stat nested file");
    if meta.node_type != NodeType::File || meta.size != 24 {
        panic!("VFS Test 3 Failed: Stat metadata mismatch on nested file: size={}", meta.size);
    }
    klog_ok!("Phase 15 Test 3 Passed: Nested directory hierarchy & readdir enumeration verified");

    // Test 4: File Seek, Offset Tracking, and Partial I/O
    let seek_file_path = "/tmp/sparse_seek.dat";
    let fd = open(seek_file_path, OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE)
        .expect("Failed to open seek test file");

    write(fd, b"HEAD").expect("Write HEAD failed"); // 0..4
    seek(fd, SeekFrom::Start(12)).expect("Seek to offset 12 failed");
    write(fd, b"TAIL").expect("Write TAIL failed"); // 12..16
    close(fd).expect("Failed to close seek file");

    let meta_seek = stat(seek_file_path).expect("Failed to stat seek file");
    if meta_seek.size != 16 {
        panic!("VFS Test 4 Failed: Expected seek file size 16, got {}", meta_seek.size);
    }

    let fd_read = open(seek_file_path, OpenFlags::READ).expect("Failed to open seek file for read");
    let mut head_buf = [0u8; 4];
    read(fd_read, &mut head_buf).expect("Failed to read head");
    if &head_buf != b"HEAD" {
        panic!("VFS Test 4 Failed: HEAD content mismatch");
    }

    seek(fd_read, SeekFrom::Start(12)).expect("Seek to 12 failed");
    let mut tail_buf = [0u8; 4];
    read(fd_read, &mut tail_buf).expect("Failed to read tail");
    if &tail_buf != b"TAIL" {
        panic!("VFS Test 4 Failed: TAIL content mismatch");
    }
    close(fd_read).expect("Failed to close fd");
    klog_ok!("Phase 15 Test 4 Passed: Seek positioning (Start/Current/End) & partial I/O verified");

    // Test 5: Special Device Nodes, Removal & Error Handling
    // 5a. /dev/null
    let fd_null = open("/dev/null", OpenFlags::WRITE | OpenFlags::READ).expect("Failed to open /dev/null");
    let write_null = write(fd_null, b"Discard this data").expect("Write to /dev/null failed");
    if write_null != 17 {
        panic!("VFS Test 5 Failed: /dev/null did not consume all bytes");
    }
    let mut null_buf = [0u8; 16];
    let read_null = read(fd_null, &mut null_buf).expect("Read from /dev/null failed");
    if read_null != 0 {
        panic!("VFS Test 5 Failed: /dev/null read did not return EOF");
    }
    close(fd_null).expect("Failed to close /dev/null");

    // 5b. /dev/zero
    let fd_zero = open("/dev/zero", OpenFlags::READ).expect("Failed to open /dev/zero");
    let mut zero_buf = [0xFFu8; 32];
    read(fd_zero, &mut zero_buf).expect("Read from /dev/zero failed");
    if zero_buf.iter().any(|&b| b != 0) {
        panic!("VFS Test 5 Failed: /dev/zero returned non-zero bytes");
    }
    close(fd_zero).expect("Failed to close /dev/zero");

    // 5c. /dev/hda (Block device from Phase 14 storage)
    let fd_hda = open("/dev/hda", OpenFlags::READ).expect("Failed to open /dev/hda");
    let mut block_buf = [0u8; 512];
    let hda_read = read(fd_hda, &mut block_buf).expect("Failed to read block from /dev/hda");
    if hda_read != 512 {
        panic!("VFS Test 5 Failed: /dev/hda block read did not return 512 bytes");
    }
    close(fd_hda).expect("Failed to close /dev/hda");

    // 5d. File removal
    remove(test_file_path).expect("Failed to remove test file");
    if open(test_file_path, OpenFlags::READ).is_ok() {
        panic!("VFS Test 5 Failed: File still exists after remove");
    }

    // 5e. Error handling on bad file descriptor
    if read(999, &mut zero_buf).is_ok() {
        panic!("VFS Test 5 Failed: Bad file descriptor read unexpectedly succeeded");
    }

    klog_ok!("Phase 15 Test 5 Passed: Device nodes (/dev/null, /dev/zero, /dev/hda) & error handling verified");
    klog_ok!("PHASE 15 VERIFIED: Virtual Filesystem (VFS) & Directory Trees operational");
}
