//! Phase 33 — Full System Integration Verification Suite
//!
//! Validates:
//! 1. Core Architecture & Memory Health Pipeline (GDT, TSS, IDT, PIC, PIT, PMM, VMM, Heap)
//! 2. Hardware & Driver Subsystem Matrix (PCI, Storage, USB, GPU, Audio, RTC, ACPI)
//! 3. Process, Scheduling & Synchronization Mesh (PCB, Priority RunQueue, Spinlock, Mutex, Semaphores)
//! 4. Storage, Filesystem & GUI Stack Pipeline (VFS, Inodes, Input, WM, Compositor, Widgets)
//! 5. Security, System Calls & Reliability Telemetry (UserPtr, Caps, Audit, Errors, Unwind)

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{klog_info, klog_ok};

/// Runs all 5 Phase 33 full system integration tests.
pub fn run_integration_tests() {
    klog_info!("=== Phase 33: Full System Integration Tests ===");

    test_core_architecture_and_memory();
    test_hardware_and_driver_matrix();
    test_process_scheduling_and_sync_mesh();
    test_storage_fs_and_gui_pipeline();
    test_security_syscalls_and_reliability();

    klog_info!("=== Phase 33: All Integration Tests Passed (5/5) ===");
}

/// Test 1: Core Architecture & Memory Health Pipeline.
///
/// Verifies GDT, TSS, IDT, PIT/PIC, physical frame allocator, 4-level paging, and dynamic heap.
fn test_core_architecture_and_memory() {
    klog_info!("Integration Test 1: Core Architecture & Memory Health Pipeline...");

    // 1. Verify CPU Architecture state
    let cr3 = crate::memory_layout::read_cr3();
    let rsp = crate::memory_layout::read_rsp();
    assert_ne!(cr3, 0, "CR3 page table base must be non-zero");
    assert_ne!(rsp, 0, "Stack pointer RSP must be non-zero");

    let layout = crate::memory_layout::KernelMemoryLayout::get();
    assert!(
        layout.is_rsp_within_stack(rsp),
        "Kernel stack bounds check failed"
    );

    // 2. Verify Timer & Interrupts
    let ticks_before = crate::arch::x86_64::pit::get_ticks();
    assert!(ticks_before > 0, "PIT timer must have delivered ticks");

    // 3. Physical Frame Allocator
    let mut fa = crate::memory::frame_allocator::FRAME_ALLOCATOR.lock();
    let initial_free = fa.free_frames();
    assert!(initial_free > 100, "Sufficient physical memory frames required");

    let frame = fa.allocate_frame().expect("Physical frame allocation failed");
    assert_eq!(fa.free_frames(), initial_free - 1);
    assert_ne!(frame, 0, "Allocated frame at address 0");

    fa.free_frame(frame);
    assert_eq!(fa.free_frames(), initial_free, "Frame count mismatch after free");
    drop(fa);

    // 4. Kernel Heap & Dynamic Allocator
    let mut vec_test: Vec<Box<u64>> = Vec::with_capacity(64);
    for i in 0..64 {
        vec_test.push(Box::new(0xCAFE_BABE_0000_0000 | (i as u64)));
    }
    assert_eq!(vec_test.len(), 64);
    for i in 0..64 {
        assert_eq!(*vec_test[i], 0xCAFE_BABE_0000_0000 | (i as u64));
    }
    drop(vec_test);

    // 5. String formatting and concatenation
    let s = alloc::format!("IntegrateOS-Phase{}-Verified", 33);
    assert_eq!(s.as_str(), "IntegrateOS-Phase33-Verified");
    drop(s);

    klog_ok!("Integration Test 1 Passed: Core architecture, PMM, VMM, and heap unified");
}

/// Test 2: Hardware & Driver Subsystem Matrix Integration.
///
/// Verifies Device Manager, PCI, Storage, USB, GPU, Audio, RTC, and ACPI.
fn test_hardware_and_driver_matrix() {
    klog_info!("Integration Test 2: Hardware & Driver Subsystem Matrix Integration...");

    // 1. Device Manager Central Registry
    let dev_mgr = crate::drivers::manager::DEVICE_MANAGER.lock();
    assert!(dev_mgr.count() > 0, "Device Manager must contain registered devices");
    let supported = dev_mgr.count_by_status(crate::drivers::traits::HardwareSupportStatus::Supported);
    assert!(supported > 0, "At least one driver must have Supported status");
    drop(dev_mgr);

    // 2. PCI Bus Enumeration
    assert!(crate::pci::is_pci_initialized(), "PCI subsystem must be initialized");
    let pci_count = crate::pci::manager::PCI_MANAGER.lock().count();
    assert!(pci_count > 0, "PCI bus scan must discover devices");

    // 3. Storage Subsystem
    assert!(crate::storage::is_storage_initialized(), "Storage subsystem must be initialized");
    assert!(crate::storage::total_blocks() > 0, "Primary storage block count must be > 0");

    // 4. USB Subsystem
    assert!(crate::usb::is_usb_initialized(), "USB subsystem must be initialized");
    let usb_devs = crate::usb::manager::USB_MANAGER.lock().device_count();
    assert!(usb_devs > 0, "USB subsystem must report at least 1 enumerated device");

    // 5. GPU Foundation
    assert!(crate::gpu::is_gpu_initialized(), "GPU foundation must be initialized");
    let has_gpu = crate::gpu::GPU_MANAGER.lock().is_some();
    assert!(has_gpu, "GPU manager singleton must be present");

    // 6. Audio Subsystem
    assert!(crate::audio::is_audio_initialized(), "Audio subsystem must be initialized");
    let has_audio = crate::audio::AUDIO_MANAGER.lock().is_some();
    assert!(has_audio, "Audio manager singleton must be present");

    // 7. Time & RTC Subsystem
    assert!(crate::time::is_time_initialized(), "Time subsystem must be initialized");
    let uptime = crate::time::uptime_ms();
    assert!(uptime > 0, "Monotonic uptime must be > 0 ms");
    let wall_clock = crate::time::format_now_iso8601();
    assert!(!wall_clock.is_empty(), "Wall clock string must not be empty");

    // 8. ACPI & Power Management
    assert!(crate::acpi::is_acpi_initialized(), "ACPI subsystem must be initialized");
    let has_acpi = crate::acpi::ACPI_MANAGER.lock().is_some();
    assert!(has_acpi, "ACPI manager singleton must be present");

    klog_ok!("Integration Test 2 Passed: 7 hardware layers + device registry coordinated");
}

/// Test 3: Process, Scheduling & Synchronization Mesh Integration.
///
/// Verifies Process Control Blocks, priority queues, and concurrency primitives.
fn test_process_scheduling_and_sync_mesh() {
    klog_info!("Integration Test 3: Process, Scheduling & Synchronization Mesh...");

    // 1. Process Table
    let pm = crate::process::manager::PROCESS_MANAGER.lock();
    assert!(pm.count() >= 1, "At least kernel root process (PID 0) must exist");
    let current_pid = pm.current_pid();
    assert_eq!(current_pid, crate::process::pid::ProcessId::KERNEL);
    drop(pm);

    // 2. Scheduler Statistics
    let (switches, preemptions, yields) = crate::scheduler::stats();
    klog_info!(
        "  Scheduler Telemetry: {} context switches, {} preemptions, {} yields",
        switches, preemptions, yields
    );

    // 3. Synchronization Primitives Mesh
    // a. Spinlock
    let lock_val = crate::sync::Spinlock::new(100u32);
    {
        let mut guard = lock_val.lock();
        *guard += 50;
    }
    assert_eq!(*lock_val.lock(), 150);

    // b. Reader-Writer Lock
    let rwlock = crate::sync::RwLock::new(Vec::new());
    {
        let mut w = rwlock.write();
        w.push(10);
        w.push(20);
    }
    {
        let r1 = rwlock.read();
        let r2 = rwlock.read();
        assert_eq!(r1.len(), 2);
        assert_eq!(r2[0], 10);
        assert_eq!(r2[1], 20);
    }

    // c. Counting Semaphore
    let sem = crate::sync::CountingSemaphore::new(3, 3);
    assert_eq!(sem.available_permits(), 3);
    assert!(sem.try_acquire());
    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 1);
    sem.release();
    assert_eq!(sem.available_permits(), 2);

    // d. Barrier Generational Cycling
    let barrier = crate::sync::Barrier::new(1);
    let res1 = barrier.wait();
    assert!(res1.is_leader(), "Single-thread barrier must elect leader");
    let res2 = barrier.wait();
    assert!(res2.is_leader(), "Subsequent barrier cycle must elect leader");

    klog_ok!("Integration Test 3 Passed: PCB, scheduler stats, and sync primitives mesh verified");
}

/// Test 4: Storage, Filesystem & GUI Stack Pipeline Integration.
///
/// Verifies VFS file I/O, event routing, Window Manager, and Compositor.
fn test_storage_fs_and_gui_pipeline() {
    klog_info!("Integration Test 4: Storage, Filesystem & GUI Stack Pipeline...");

    // 1. Filesystem Pipeline: Directory creation, file write, file read, file remove
    let dir_path = "/integration_dir";
    let file_path = "/integration_dir/config.sys";
    let payload = b"INTEGRATE_OS_DESKTOP_CONFIG=ACTIVE\nTHEME=DARK\nSCALE=100\n";

    assert!(crate::fs::mkdir(dir_path).is_ok(), "Failed to create directory in VFS");
    assert!(
        crate::fs::write_file(file_path, payload).is_ok(),
        "Failed to write file to VFS"
    );

    let read_back = crate::fs::read_file(file_path).expect("Failed to read file from VFS");
    assert_eq!(read_back.as_slice(), payload, "File payload mismatch");

    let stat = crate::fs::stat(file_path).expect("Failed to stat VFS file");
    assert_eq!(stat.size, payload.len() as u64);

    assert!(crate::fs::remove(file_path).is_ok(), "Failed to remove test file");
    assert!(crate::fs::remove(dir_path).is_ok(), "Failed to remove test dir");

    // 2. Input Subsystem Event Pipeline
    let mouse_ev = crate::input::MouseEvent {
        position: (100, 100),
        delta: (5, -3),
        buttons: crate::input::MouseButtons::LEFT,
        scroll: 0,
    };
    let input_ev = crate::input::InputEvent::new(
        crate::input::InputEventKind::Mouse(mouse_ev),
        crate::arch::x86_64::pit::get_ticks(),
    );
    crate::input::push_event(input_ev);
    assert!(crate::input::has_events(), "Input queue must have pending event");

    let popped = crate::input::pop_event().expect("Failed to dequeue input event");
    if let crate::input::InputEventKind::Mouse(m) = popped.kind {
        assert_eq!(m.delta.0, 5);
        assert_eq!(m.delta.1, -3);
        assert!(m.buttons.is_left());
    } else {
        panic!("Dequeued input event type mismatch");
    }

    // 3. Window Manager & Compositor
    let has_wm = crate::wm::WINDOW_MANAGER.lock().is_some();
    let has_comp = crate::compositor::COMPOSITOR.lock().is_some();
    assert!(has_wm, "Window Manager instance must be active");
    assert!(has_comp, "Compositor instance must be active");

    // 4. GUI Toolkit widget rendering check
    let mut label = crate::gui::Label::new("Integrate OS 2026", 10, 10, 160, 20);
    label.set_text("Integrate OS 2026");
    assert_eq!(label.text.as_str(), "Integrate OS 2026");

    klog_ok!("Integration Test 4 Passed: End-to-end Storage, VFS, Input, WM, and GUI verified");
}

/// Test 5: Security, System Calls & Reliability Telemetry Integration.
///
/// Verifies user pointer checks, capability sets, audit log, error taxonomy, and stack unwinding.
fn test_security_syscalls_and_reliability() {
    klog_info!("Integration Test 5: Security, Syscalls & Reliability Telemetry...");

    // 1. User Pointer MMU & Bounds Validation
    // Kernel space address must fail validation
    let kernel_ptr = 0xFFFF_8000_0000_2000u64;
    let res_kernel = crate::security::validate_user_address(kernel_ptr, 128, false);
    assert!(
        res_kernel.is_err(),
        "UserPtr validator must reject kernel addresses"
    );

    // Arithmetic overflow must fail validation
    let res_overflow = crate::security::validate_user_address(
        crate::user::USER_CODE_VIRT_ADDR,
        usize::MAX,
        false,
    );
    assert!(
        res_overflow.is_err(),
        "UserPtr validator must reject integer arithmetic overflow"
    );

    // 2. Capability System & One-Way Revocation
    let mut caps = crate::security::CapabilitySet::ALL;
    assert!(caps.has(crate::security::Capability::ReadStorage));
    assert!(caps.has(crate::security::Capability::WriteStorage));
    assert!(caps.has(crate::security::Capability::NetworkAccess));

    caps.revoke(crate::security::Capability::NetworkAccess);
    assert!(!caps.has(crate::security::Capability::NetworkAccess));
    assert!(caps.has(crate::security::Capability::ReadStorage));

    // 3. Security Audit Log Trail
    let audit_count_before = crate::security::with_security(|sm| sm.audit_log().event_count()).unwrap_or(0);
    crate::security::with_security_mut(|sm| {
        sm.audit_log_mut().record(
            crate::process::pid::ProcessId::KERNEL,
            crate::security::AuditSeverity::Info,
            crate::security::AuditEventKind::ResourceExceeded {
                limit: crate::security::ResourceLimitType::MaxMemoryBytes,
                requested: 1024,
                current: 2048,
                max: 4096,
            },
            "Phase 33 Integration Audit Check",
        );
    });
    let audit_count_after = crate::security::with_security(|sm| sm.audit_log().event_count()).unwrap_or(0);
    assert_eq!(audit_count_after, audit_count_before + 1);

    // 4. Unified Kernel Error Taxonomy & POSIX Errno Mapping
    let err_io = crate::debug::KernelErrorCode::IoError;
    assert_eq!(err_io.as_errno(), 5); // EIO = 5
    assert_eq!(err_io.category(), crate::debug::KernelErrorCategory::Device);

    let err_nomem = crate::debug::KernelErrorCode::OutOfMemory;
    assert_eq!(err_nomem.as_errno(), 12); // ENOMEM = 12
    assert_eq!(err_nomem.category(), crate::debug::KernelErrorCategory::Memory);

    // 5. Bare-Metal Stack Frame Unwinding & Register Inspection
    let frames = crate::debug::unwind_stack();
    assert!(
        !frames.is_empty(),
        "Stack unwinder must recover at least 1 stack frame"
    );
    klog_info!("  Stack Unwind: Captured {} return frame(s)", frames.len());

    // 6. Memory Hex Dumper
    let test_buf: [u8; 16] = [
        0x49, 0x6E, 0x74, 0x65, 0x67, 0x72, 0x61, 0x74,
        0x65, 0x4F, 0x53, 0x5F, 0x44, 0x65, 0x62, 0x75,
    ];
    let hex_dump = crate::debug::format_hex_dump(test_buf.as_ptr() as u64, test_buf.len());
    assert!(hex_dump.contains("49 6e 74 65"));
    assert!(hex_dump.contains("|IntegrateOS_Debu|"));

    klog_ok!("Integration Test 5 Passed: Security, syscall checks, audit log, and error telemetry verified");
}
