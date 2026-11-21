//! Phase 32 — Debugging + Reliability Verification Suite
//!
//! Validates:
//! 1. Structured logging engine & dynamic log level threshold filtering
//! 2. In-memory circular log ring buffer capacity wrap-around & recent history extraction
//! 3. Unified kernel error codes, subsystem taxonomy & POSIX errno bidirectional mapping
//! 4. Architectural CPU exception error code decoding (#PF bitmask flags & #GP descriptor selectors)
//! 5. Stack frame pointer chain unwinding, register readout & memory hex dumping

use crate::debug::errors::{KernelErrorCategory, KernelErrorCode};
use crate::debug::exception::{exception_name, GeneralProtectionError, PageFaultError, TableIndicator};
use crate::debug::logger::{get_max_level, is_level_enabled, set_max_level};
use crate::debug::panic::{format_hex_dump, read_rbp, read_rsp, unwind_stack};
use crate::debug::ring_buffer::{LogLevel, LogRingBuffer, LOG_RING_BUFFER_CAPACITY};
use crate::{klog_debug, klog_error, klog_info, klog_warn};

/// Runs all 5 Phase 32 verification tests.
pub fn run_debug_tests() {
    klog_info!("=== Phase 32: Debugging + Reliability Tests ===");

    test_logging_and_filtering();
    test_log_ring_buffer();
    test_unified_error_codes();
    test_exception_decoding();
    test_stack_unwinding_and_hexdump();

    klog_info!("=== Phase 32: All Debugging & Reliability Tests Passed (5/5) ===");
}

/// Test 1: Structured logging & dynamic log level threshold filtering.
fn test_logging_and_filtering() {
    klog_info!("Debug Test 1: Structured Logging & Dynamic Level Filtering...");

    let original_level = get_max_level();

    // 1. Set threshold to Warn
    set_max_level(LogLevel::Warn);
    assert_eq!(get_max_level(), LogLevel::Warn);

    // Trace, Debug, Info must be disabled
    assert!(!is_level_enabled(LogLevel::Trace));
    assert!(!is_level_enabled(LogLevel::Debug));
    assert!(!is_level_enabled(LogLevel::Info));

    // Warn, Error, Fatal must be enabled
    assert!(is_level_enabled(LogLevel::Warn));
    assert!(is_level_enabled(LogLevel::Error));
    assert!(is_level_enabled(LogLevel::Fatal));

    // 2. Set threshold to Trace
    set_max_level(LogLevel::Trace);
    assert!(is_level_enabled(LogLevel::Trace));
    assert!(is_level_enabled(LogLevel::Debug));
    assert!(is_level_enabled(LogLevel::Info));

    // 3. Emit structured logs via macros
    klog_debug!("kernel::test", "Diagnostic debug trace message");
    klog_warn!("kernel::test", "Telemetry warning message");
    klog_error!("kernel::test", "Simulated recoverable error message");

    // Restore original level
    set_max_level(original_level);

    klog_info!("Debug Test 1: PASS — Dynamic logging threshold & severity filtering verified");
}

/// Test 2: In-memory circular log ring buffer capacity wrap-around & recent history extraction.
fn test_log_ring_buffer() {
    klog_info!("Debug Test 2: Diagnostic Log Ring Buffer & Retention...");

    let mut rb = LogRingBuffer::new();
    assert_eq!(rb.len(), 0);
    assert!(rb.is_empty());

    // 1. Push entries up to capacity + 50 to force circular wrap-around
    let total_push = LOG_RING_BUFFER_CAPACITY + 50;
    for i in 0..total_push {
        rb.push(
            LogLevel::Info,
            "kernel::ring_test",
            alloc::format!("Log entry #{}", i),
        );
    }

    // 2. Length must be capped at LOG_RING_BUFFER_CAPACITY
    assert_eq!(rb.len(), LOG_RING_BUFFER_CAPACITY);
    assert!(!rb.is_empty());

    // 3. Recent entries inspection
    let recent_5 = rb.recent(5);
    assert_eq!(recent_5.len(), 5);

    // Oldest in the recent 5 should be index (total_push - 5)
    let last_entry = &recent_5[4];
    assert_eq!(last_entry.message, alloc::format!("Log entry #{}", total_push - 1));
    assert_eq!(last_entry.level, LogLevel::Info);
    assert_eq!(last_entry.target, "kernel::ring_test");

    // 4. Clear buffer
    rb.clear();
    assert_eq!(rb.len(), 0);
    assert!(rb.is_empty());

    klog_info!("Debug Test 2: PASS — Circular buffer wrap-around (cap={}) & retention verified", LOG_RING_BUFFER_CAPACITY);
}

/// Test 3: Unified kernel error codes, subsystem taxonomy & POSIX errno translation.
fn test_unified_error_codes() {
    klog_info!("Debug Test 3: Unified Kernel Error Codes & Errno Mapping...");

    // 1. OutOfMemory mapping
    let enomem = KernelErrorCode::OutOfMemory;
    assert_eq!(enomem.as_errno(), 12);
    assert_eq!(enomem.category(), KernelErrorCategory::Memory);
    assert_eq!(KernelErrorCode::from_errno(12), Some(KernelErrorCode::OutOfMemory));
    assert!(!enomem.description().is_empty());

    // 2. PermissionDenied mapping
    let eacces = KernelErrorCode::PermissionDenied;
    assert_eq!(eacces.as_errno(), 13);
    assert_eq!(eacces.category(), KernelErrorCategory::Security);
    assert_eq!(KernelErrorCode::from_errno(13), Some(KernelErrorCode::PermissionDenied));

    // 3. BadAddress mapping
    let efault = KernelErrorCode::BadAddress;
    assert_eq!(efault.as_errno(), 14);
    assert_eq!(efault.category(), KernelErrorCategory::Memory);
    assert_eq!(KernelErrorCode::from_errno(14), Some(KernelErrorCode::BadAddress));

    // 4. BrokenPipe network error
    let epipe = KernelErrorCode::BrokenPipe;
    assert_eq!(epipe.as_errno(), 32);
    assert_eq!(epipe.category(), KernelErrorCategory::Network);

    // 5. Invalid errno lookup returns None
    assert_eq!(KernelErrorCode::from_errno(9999), None);

    klog_info!("Debug Test 3: PASS — POSIX errno roundtrip & error taxonomy verified");
}

/// Test 4: Architectural CPU exception error code decoding (#PF & #GP).
fn test_exception_decoding() {
    klog_info!("Debug Test 4: CPU Exception Error Code Decoding...");

    // 1. Page Fault decoding: Supervisor read of non-present page (code = 0)
    let pf0 = PageFaultError::from_raw(0);
    assert!(!pf0.present);
    assert!(!pf0.write);
    assert!(!pf0.user_mode);
    assert!(!pf0.instruction_fetch);

    // 2. Page Fault decoding: User mode write to protected present page (code = 0b00111 = 7)
    let pf7 = PageFaultError::from_raw(7);
    assert!(pf7.present);
    assert!(pf7.write);
    assert!(pf7.user_mode);
    assert!(!pf7.instruction_fetch);

    // 3. Page Fault decoding: User mode instruction fetch violation on NX page (code = 0b10101 = 21)
    let pf21 = PageFaultError::from_raw(21);
    assert!(pf21.present);
    assert!(!pf21.write);
    assert!(pf21.user_mode);
    assert!(pf21.instruction_fetch);

    // 4. General Protection Fault decoding: GDT selector index 4 (0x20: index=4, TI=0, IDT=0, EXT=0)
    let gp_code = 0x20;
    let gp = GeneralProtectionError::from_raw(gp_code);
    assert_eq!(gp.selector_index, 4);
    assert_eq!(gp.table, TableIndicator::Gdt);
    assert!(!gp.idt);
    assert!(!gp.external);

    // 5. GP Fault decoding: IDT gate selector index 14, external interrupt (code = (14 << 3) | 0b011)
    let gp_idt_code = (14 << 3) | 0b011;
    let gp_idt = GeneralProtectionError::from_raw(gp_idt_code);
    assert_eq!(gp_idt.selector_index, 14);
    assert!(gp_idt.idt);
    assert!(gp_idt.external);

    // 6. Exception vector names
    assert_eq!(exception_name(14), "#PF (Page Fault)");
    assert_eq!(exception_name(13), "#GP (General Protection Fault)");
    assert_eq!(exception_name(6), "#UD (Invalid Opcode)");

    klog_info!("Debug Test 4: PASS — #PF bitmask decoding, #GP descriptor parsing & vector taxonomy verified");
}

/// Test 5: Stack frame pointer chain unwinding, register readout & memory hex dumping.
fn test_stack_unwinding_and_hexdump() {
    klog_info!("Debug Test 5: Stack Unwinding & Memory Hex Dump...");

    // 1. Register readouts
    let rsp = read_rsp();
    let rbp = read_rbp();
    assert_ne!(rsp, 0, "RSP must be non-zero");
    assert_ne!(rbp, 0, "RBP must be non-zero");
    assert_eq!(rsp & 7, 0, "RSP must be 8-byte aligned on function entry");

    // 2. Unwind current stack frames
    let frames = unwind_stack();
    assert!(!frames.is_empty(), "Stack unwinding must capture active call frames");

    for frame in &frames {
        assert_ne!(frame.rip, 0, "Frame return RIP must be non-zero");
        assert_ne!(frame.rbp, 0, "Frame pointer RBP must be non-zero");
    }

    // 3. Hex dumper verification
    let sample_bytes: [u8; 32] = [
        0x49, 0x6e, 0x74, 0x65, 0x67, 0x72, 0x61, 0x74,
        0x65, 0x4f, 0x53, 0x5f, 0x44, 0x65, 0x62, 0x75,
        0x67, 0x54, 0x65, 0x73, 0x74, 0x5f, 0x32, 0x30,
        0x32, 0x36, 0x5f, 0x4f, 0x4b, 0x21, 0x00, 0xff,
    ];
    let dump = format_hex_dump(sample_bytes.as_ptr() as u64, sample_bytes.len());
    assert!(!dump.is_empty());
    assert!(dump.contains("|IntegrateOS_Debu|"));
    assert!(dump.contains("|gTest_2026_OK!..|"));

    klog_info!(
        "Debug Test 5: PASS — Stack frames (depth={}), RSP={:#x}, RBP={:#x} & hex dump verified",
        frames.len(),
        rsp,
        rbp
    );
}
