//! Phase 30 — Security + Isolation Verification Suite
//!
//! Validates:
//! 1. User memory pointer validation & kernel boundary protection (NULL, low BIOS, canonical overflow, supervisor page)
//! 2. Capability-based access control & one-way privilege dropping (revocation & elevation defense)
//! 3. Process resource accounting & quota enforcement (memory, file descriptors, child processes, CPU time)
//! 4. Strict sandbox & multi-tier process isolation (zero hardware I/O, zero storage mutation, zero spawn)
//! 5. Security audit logging & fault containment (structured audit trail, denial/fault tracking, zero panic)

use crate::klog_info;
use crate::process::pid::ProcessId;
use crate::security::manager::{with_security, with_security_mut};
use crate::security::pointer::{
    validate_user_address, UserPtr, UserSlice, KERNEL_SPACE_MIN, USER_SPACE_MAX,
};
use crate::security::sandbox::SandboxPolicy;
use crate::security::types::{
    Capability, PointerViolation, ResourceLimitType, ResourceLimits, SecurityContext,
    SecurityError,
};
use crate::user::USER_CODE_VIRT_ADDR;

/// Runs all 5 Phase 30 verification tests and logs results.
pub fn run_security_tests() {
    klog_info!("=== Phase 30: Security + Isolation Tests ===");

    test_user_pointer_validation();
    test_capability_access_control();
    test_resource_limits_and_accounting();
    test_sandbox_isolation();
    test_audit_logging_and_containment();

    klog_info!("=== Phase 30: All Security Tests Passed (5/5) ===");
}

/// Test 1: User memory pointer validation & kernel boundary protection.
fn test_user_pointer_validation() {
    klog_info!("Security Test 1: User Pointer Validation & Boundary Protection...");

    // 1. NULL pointer rejection
    let null_res = validate_user_address(0x0, 4, false);
    assert_eq!(
        null_res,
        Err(PointerViolation::NullPointer),
        "NULL pointer must be rejected"
    );

    // 2. Low reserved BIOS memory (< 64 KiB) rejection
    let low_mem_res = validate_user_address(0x0400, 16, false);
    assert_eq!(
        low_mem_res,
        Err(PointerViolation::UnderflowLowMemory),
        "Low BIOS memory (< 64 KiB) must be rejected"
    );

    // 3. Arithmetic overflow rejection
    let overflow_res = validate_user_address(USER_CODE_VIRT_ADDR, usize::MAX, false);
    assert_eq!(
        overflow_res,
        Err(PointerViolation::ArithmeticOverflow),
        "Arithmetic overflow must be rejected"
    );

    // 4. Non-canonical / higher-half supervisor address rejection
    let kernel_high_res = validate_user_address(KERNEL_SPACE_MIN, 8, false);
    assert_eq!(
        kernel_high_res,
        Err(PointerViolation::KernelAddressViolation),
        "Higher-half supervisor space must be rejected"
    );

    let user_max_res = validate_user_address(USER_SPACE_MAX + 1, 8, false);
    assert_eq!(
        user_max_res,
        Err(PointerViolation::KernelAddressViolation),
        "Address exceeding canonical user space must be rejected"
    );

    // 5. Supervisor-mapped page rejection (1 MiB kernel identity map)
    let supervisor_res = validate_user_address(0x0010_0000, 4, false);
    assert_eq!(
        supervisor_res,
        Err(PointerViolation::SupervisorPage { addr: 0x0010_0000 }),
        "Supervisor-only page must be rejected"
    );

    // 6. Unmapped user virtual address rejection (8 GiB: outside 4 GiB identity map, inside 128 TiB canonical user space)
    let unmapped_addr = 0x0000_0002_0000_0000;
    let unmapped_res = validate_user_address(unmapped_addr, 4, false);
    assert_eq!(
        unmapped_res,
        Err(PointerViolation::PageNotPresent { addr: unmapped_addr }),
        "Unmapped page must be rejected"
    );

    // 7. Legitimate mapped user address (USER_CODE_VIRT_ADDR mapped in Phase 10)
    let valid_read = validate_user_address(USER_CODE_VIRT_ADDR, 4, false);
    assert!(
        valid_read.is_ok(),
        "Mapped user page must pass read validation"
    );

    // 8. Typed UserPtr read verification
    let uptr = UserPtr::<u32>::new(USER_CODE_VIRT_ADDR).expect("UserPtr creation should succeed");
    let val = uptr.read().expect("UserPtr read should succeed");
    assert_eq!(
        uptr.addr(),
        USER_CODE_VIRT_ADDR,
        "UserPtr address must match"
    );

    // 9. Unaligned pointer rejection
    let unaligned_res = UserPtr::<u32>::new(USER_CODE_VIRT_ADDR + 1);
    assert_eq!(
        unaligned_res.err(),
        Some(PointerViolation::Unaligned { required: 4, actual: 1 }),
        "Unaligned pointer must be rejected"
    );

    // 10. UserSlice verification
    let slice = UserSlice::<u8>::new(USER_CODE_VIRT_ADDR, 16).expect("UserSlice creation should succeed");
    let mut kernel_buf = [0u8; 16];
    let copy_res = slice.copy_to_slice(&mut kernel_buf);
    assert!(copy_res.is_ok(), "UserSlice copy_to_slice should succeed");

    klog_info!(
        "Security Test 1: PASS — Boundary checks, MMU page flags, UserPtr/UserSlice verified (sample={:#x})",
        val
    );
}

/// Test 2: Capability-based access control & privilege revocation.
fn test_capability_access_control() {
    klog_info!("Security Test 2: Capability-Based Access Control & Privilege Revocation...");

    let test_pid = ProcessId(9101);
    let ctx = SecurityContext::new_user(test_pid, 1000, 1000, "test_cap_proc");

    with_security_mut(|sm| {
        sm.register_process(ctx);
    });

    // 1. Verify standard capabilities are present
    let read_storage = with_security_mut(|sm| sm.check_capability(test_pid, Capability::ReadStorage, "read_vfs"));
    assert_eq!(read_storage, Some(Ok(())), "Standard user has ReadStorage");

    let net_access = with_security_mut(|sm| sm.check_capability(test_pid, Capability::NetworkAccess, "socket_open"));
    assert_eq!(net_access, Some(Ok(())), "Standard user has NetworkAccess");

    // 2. Verify privileged capability is absent
    let hw_io = with_security_mut(|sm| sm.check_capability(test_pid, Capability::HardwareIo, "port_io"));
    assert_eq!(
        hw_io,
        Some(Err(SecurityError::MissingCapability(Capability::HardwareIo))),
        "Standard user lacks HardwareIo"
    );

    // 3. Drop NetworkAccess permanently
    let drop_res = with_security_mut(|sm| sm.drop_capability(test_pid, Capability::NetworkAccess));
    assert_eq!(drop_res, Some(Ok(())), "Drop capability should succeed");

    // 4. Verify NetworkAccess is now revoked
    let net_denied = with_security_mut(|sm| sm.check_capability(test_pid, Capability::NetworkAccess, "socket_open"));
    assert_eq!(
        net_denied,
        Some(Err(SecurityError::MissingCapability(Capability::NetworkAccess))),
        "NetworkAccess must be denied after revocation"
    );

    // 5. Verify ReadStorage is still held
    let read_still_ok = with_security_mut(|sm| sm.check_capability(test_pid, Capability::ReadStorage, "read_vfs"));
    assert_eq!(read_still_ok, Some(Ok(())), "ReadStorage retained after selective drop");

    // 6. Attempt privilege escalation: re-granting dropped capability
    let elevation_attempt = with_security_mut(|sm| {
        let ctx = sm.get_context_mut(test_pid).unwrap();
        let mut new_set = ctx.capabilities;
        new_set.grant(Capability::NetworkAccess);
        ctx.set_capabilities(new_set)
    });
    assert_eq!(
        elevation_attempt,
        Some(Err(SecurityError::PrivilegeElevationForbidden)),
        "Privilege elevation beyond bounding set must be rejected"
    );

    klog_info!("Security Test 2: PASS — Capability checks, one-way dropping & anti-elevation verified");
}

/// Test 3: Process resource accounting & quota enforcement.
fn test_resource_limits_and_accounting() {
    klog_info!("Security Test 3: Resource Limits & Accounting Enforcement...");

    let test_pid = ProcessId(9102);
    let limits = ResourceLimits::new(64 * 1024, 2, 1, 50); // 64 KiB, 2 files, 1 child, 50 ms
    let mut ctx = SecurityContext::new_user(test_pid, 1000, 1000, "test_limits_proc");
    ctx.limits = limits;

    with_security_mut(|sm| {
        sm.register_process(ctx);
    });

    // 1. Memory quota
    let mem1 = with_security_mut(|sm| sm.charge_memory(test_pid, 32 * 1024));
    assert_eq!(mem1, Some(Ok(())), "Allocating 32 KiB under 64 KiB limit succeeds");

    let mem2 = with_security_mut(|sm| sm.charge_memory(test_pid, 48 * 1024));
    assert_eq!(
        mem2,
        Some(Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxMemoryBytes))),
        "Allocating 48 KiB more exceeds 64 KiB quota"
    );

    with_security_mut(|sm| sm.release_memory(test_pid, 32 * 1024));

    // 2. Open file descriptor quota
    let f1 = with_security_mut(|sm| sm.charge_file(test_pid));
    assert_eq!(f1, Some(Ok(())), "Open file 1 succeeds");

    let f2 = with_security_mut(|sm| sm.charge_file(test_pid));
    assert_eq!(f2, Some(Ok(())), "Open file 2 succeeds");

    let f3 = with_security_mut(|sm| sm.charge_file(test_pid));
    assert_eq!(
        f3,
        Some(Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxOpenFiles))),
        "Opening file 3 exceeds limit of 2"
    );

    with_security_mut(|sm| sm.release_file(test_pid));
    let f3_retry = with_security_mut(|sm| sm.charge_file(test_pid));
    assert_eq!(f3_retry, Some(Ok(())), "Opening file succeeds after closing a file");

    // 3. Child process quota
    let p1 = with_security_mut(|sm| sm.charge_process(test_pid));
    assert_eq!(p1, Some(Ok(())), "Spawning child 1 succeeds");

    let p2 = with_security_mut(|sm| sm.charge_process(test_pid));
    assert_eq!(
        p2,
        Some(Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxChildProcesses))),
        "Spawning child 2 exceeds limit of 1"
    );

    with_security_mut(|sm| sm.release_process(test_pid));
    let p2_retry = with_security_mut(|sm| sm.charge_process(test_pid));
    assert_eq!(p2_retry, Some(Ok(())), "Spawning child succeeds after releasing");

    // 4. CPU execution time quota
    let cpu1 = with_security_mut(|sm| sm.consume_cpu_time(test_pid, 30));
    assert_eq!(cpu1, Some(Ok(())), "Consuming 30 ms under 50 ms quota succeeds");

    let cpu2 = with_security_mut(|sm| sm.consume_cpu_time(test_pid, 30));
    assert_eq!(
        cpu2,
        Some(Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxCpuTimeMs))),
        "Consuming 30 ms more exceeds 50 ms quota"
    );

    klog_info!("Security Test 3: PASS — Memory, file descriptor, process & CPU limits enforced");
}

/// Test 4: Strict Sandbox & Multi-Tier Process Isolation.
fn test_sandbox_isolation() {
    klog_info!("Security Test 4: Strict Sandbox & Multi-Tier Process Isolation...");

    let test_pid = ProcessId(9103);
    let ctx = SecurityContext::new_sandboxed(test_pid, "isolated_sandbox");

    with_security_mut(|sm| {
        sm.register_process(ctx);
    });

    // 1. Verify sandboxed state
    let is_sandboxed = with_security(|sm| sm.get_context(test_pid).map(|c| c.is_sandboxed));
    assert_eq!(is_sandboxed, Some(Some(true)), "Process marked as sandboxed");

    // 2. Hardware I/O blocked
    let hw = with_security_mut(|sm| sm.check_capability(test_pid, Capability::HardwareIo, "raw_inb"));
    assert_eq!(
        hw,
        Some(Err(SecurityError::MissingCapability(Capability::HardwareIo))),
        "Sandboxed process denied HardwareIo"
    );

    // 3. Storage mutation blocked
    let write_disk = with_security_mut(|sm| sm.check_capability(test_pid, Capability::WriteStorage, "vfs_write"));
    assert_eq!(
        write_disk,
        Some(Err(SecurityError::MissingCapability(Capability::WriteStorage))),
        "Sandboxed process denied WriteStorage"
    );

    // 4. Network exfiltration blocked
    let net = with_security_mut(|sm| sm.check_capability(test_pid, Capability::NetworkAccess, "connect_tcp"));
    assert_eq!(
        net,
        Some(Err(SecurityError::MissingCapability(Capability::NetworkAccess))),
        "Sandboxed process denied NetworkAccess"
    );

    // 5. Process management blocked
    let proc_ctrl = with_security_mut(|sm| sm.check_capability(test_pid, Capability::ManageProcesses, "spawn_proc"));
    assert_eq!(
        proc_ctrl,
        Some(Err(SecurityError::MissingCapability(Capability::ManageProcesses))),
        "Sandboxed process denied ManageProcesses"
    );

    // 6. Child process creation quota is zero
    let spawn_res = with_security_mut(|sm| sm.charge_process(test_pid));
    assert_eq!(
        spawn_res,
        Some(Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxChildProcesses))),
        "Sandboxed process child limit is 0"
    );

    // 7. IPC communication is permitted (for client-server interaction)
    let ipc = with_security_mut(|sm| sm.check_capability(test_pid, Capability::IpcCommunication, "ipc_send"));
    assert_eq!(ipc, Some(Ok(())), "Sandboxed process retains IPC capability");

    // 8. Custom Sandbox Policy: READ_ONLY_FS
    let ro_pid = ProcessId(9104);
    let mut ro_ctx = SecurityContext::new_user(ro_pid, 1000, 1000, "ro_app");
    SandboxPolicy::READ_ONLY_FS.apply(&mut ro_ctx);
    with_security_mut(|sm| sm.register_process(ro_ctx));

    let ro_read = with_security_mut(|sm| sm.check_capability(ro_pid, Capability::ReadStorage, "read_vfs"));
    let ro_write = with_security_mut(|sm| sm.check_capability(ro_pid, Capability::WriteStorage, "write_vfs"));
    assert_eq!(ro_read, Some(Ok(())), "ReadOnly policy allows read");
    assert_eq!(
        ro_write,
        Some(Err(SecurityError::MissingCapability(Capability::WriteStorage))),
        "ReadOnly policy denies write"
    );

    klog_info!("Security Test 4: PASS — Strict sandbox containment & read-only isolation verified");
}

/// Test 5: Security audit logging & fault containment.
fn test_audit_logging_and_containment() {
    klog_info!("Security Test 5: Security Audit Logging & Fault Containment...");

    let audit_pid = ProcessId(9105);
    let ctx = SecurityContext::new_user(audit_pid, 1000, 1000, "audit_test_proc");
    with_security_mut(|sm| sm.register_process(ctx));

    // Record an explicit pointer fault
    with_security_mut(|sm| {
        sm.record_pointer_fault(audit_pid, 0x0, 4, PointerViolation::NullPointer);
    });

    // Inspect audit log statistics
    let (denials, faults, event_count, last_event_desc) = with_security(|sm| {
        let log = sm.audit_log();
        (
            log.total_denials(),
            log.total_faults(),
            log.event_count(),
            log.last_event().map(|e| e.description.clone()),
        )
    }).expect("Security manager must be active");

    assert!(denials > 0, "Audit log must record capability and resource denials");
    assert!(faults > 0, "Audit log must record memory pointer faults");
    assert!(event_count > 0, "Audit log must contain recorded events");
    assert!(last_event_desc.is_some(), "Audit log must have a last event");

    // Verify all recorded violations did not cause system panic or corruption
    klog_info!(
        "Security Test 5: PASS — Audit trail active (denials={}, faults={}, total_events={})",
        denials,
        faults,
        event_count
    );
}
