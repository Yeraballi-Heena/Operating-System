# Integrate OS — Changelog

## Phase 35 to 42 — Micro OS Core Components, Packaging, App Store & Release Deployment (2026-11-18)

### Added
- **Phase 35: Baseline Boot-Time Profiler & Dual Network Connectivity (LAN & WiFi)**
  - `kernel/src/time/boot_profiler.rs`: Real-time milestone tracker (`BootMilestone`, `ComponentBootMeasurement`, `log_comparison_table()`).
  - `kernel/src/net/wifi.rs`: IEEE 802.11 WiFi manager, AP discovery, 4-way WPA2 handshake, dual stack coexistence with Intel E1000 Gigabit Ethernet.
  - `kernel/src/net/phase35_tests.rs`: 5 tests + Boot Time Check #1 recorded (~6850 ms baseline).
- **Phase 36: Laptop Hardware Compatibility Architecture (Intel i7-1355U / Iris Xe Profile)**
  - `kernel/src/drivers/laptop.rs`: Hardware descriptor for `DESKTOP-L8RF961` (13th Gen Intel Core i7-1355U, 10 cores [2P+8E], 12 threads, 16GB RAM, Intel Iris Xe graphics `0x8086:0xA7A0`, NVMe PCIe SSD, ACPI battery & thermal zones).
  - `kernel/src/drivers/laptop_tests.rs`: 5 tests + Boot Time Check #2 recorded (~12 ms).
- **Phase 37: In-Built Package Manager (APTITUDE alias APT for Debian)**
  - `kernel/src/pkg/package.rs`: Debian package representation (`DebianPackage`, `PackageStatus`).
  - `kernel/src/pkg/database.rs`: dpkg status database and `/etc/apt/sources.list` seeded with 11 core Debian packages.
  - `kernel/src/pkg/manager.rs`: Full `apt` operations (`update`, `list`, `search`, `install`, `remove`, `show`) and `aptitude` alias.
  - `kernel/src/pkg/tests.rs`: 5 tests + Boot Time Check #3 recorded (~8 ms).
  - `kernel/src/apps/terminal.rs`: Added interactive `apt` and `aptitude` commands.
- **Phase 38: In-Built Store (snapstore) & Consolidated Boot Time Telemetry**
  - `kernel/src/apps/snapstore.rs`: `SnapPackage`, `SnapStoreManager` with 8 curated apps (`firefox`, `code`, `vlc`, `spotify`, `discord`, `gimp`, `wireshark`, `htop-snap`), CLI snap dispatcher, and `SnapStoreApp` GUI Software Center.
  - `kernel/src/apps/snapstore_tests.rs`: 5 tests + Boot Time Check #4 (~10 ms) + Consolidated Boot Time Comparison Table (`log_boot_comparison_table()`).
  - `kernel/src/apps/terminal.rs`: Added interactive `snap` and `snapstore` commands.
- **Phase 39: Bootable ISO Production**
  - Verified ISO-9660 PVD (LBA 16), El Torito BRVD (LBA 17), Boot Catalog (LBA 19), Stage 2 Multiboot loader (LBA 25), GRUB config (LBA 26), and kernel ELF (LBA 27).
  - `scripts/test_iso.ps1`: Automated 5-test ISO validation suite passing 100%.
- **Phase 40: Release Candidate + Headless VirtualBox Verification**
  - `scripts/rc_test.ps1`: Automated end-to-end VM test runner verifying zero panics, 32/32 subsystems, research writeup telemetry, and all 4 Micro OS components.
- **Phase 41: Portable Disk Image (RAW / VDI / IMG)**
  - `scripts/disk_image.ps1`: Generates 64 MiB raw disk image with valid MBR, partition table (0x80 Active, 0x83 Linux, LBA 2048 alignment), VBR, and kernel ELF payload; converts to `IntegrateOS.img` and VirtualBox `IntegrateOS.vdi`.
  - 5 automated verification tests passing 100%.
- **Phase 42: USB Flashing Engine (Final Phase)**
  - `scripts/flash_usb.ps1`: Physical drive enumeration, internal fixed drive safety protection (shields user's NVMe drive), block streaming write, SHA-256 verification (`72783bf1...`), and step-by-step laptop boot instructions.
  - 5 automated verification tests passing 100%.

## Phase 34 — Research Incorporation: Processor Affinity, Workload Distribution & CPU-Time Telemetry (2026-11-18)

### Added
- `kernel/src/research/affinity.rs`: Processor Affinity Architecture:
  - `CpuAffinityMask`: 64-bit affinity bitmask representing logical CPU cores `0..63`.
  - Standard predefined masks from the research writeup: `DEFAULT_ALL_ALLOWED` (`0x0000_FFFF` for 16 allowed logical cores `0..15`), `SINGLE_CPU_0` (`0x0001` for core 0), and `FOUR_CPUS_0_2_4_6` (`0x0055` for alternating cores 0, 2, 4, 6).
  - Methods: `from_bits`, `bits`, `set_cpu`, `is_cpu_allowed`, `count_allowed`, `intersect`, and `to_cpu_list_string`.
- `kernel/src/research/workload.rs`: Deterministic Computational Workload & Multi-Worker Partitioning:
  - Implements the exact deterministic CPU-intensive integer workload algorithm from the research paper: `cpu_work(iterations)` computing `result = (result + (i * 17) % 97) % 1_000_003`.
  - `run_worker_unit`: Executes individual work units, captures runtime in milliseconds, and validates mathematical checksum.
  - `execute_distributed_workload`: Partitions `4` independent work units across `1`, `2`, and `4` worker tasks, verifying checksum consistency across all units and measuring execution speedup.
- `kernel/src/research/telemetry.rs`: Research Telemetry & Experiment Reporting:
  - Data containers for research writeup tables: `AffinityExperimentRecord` (Table 1), `DistributionExperimentRecord` (Table 2), and `CpuUtilizationRecord` (Table 3).
  - `format_full_report`: Generates multi-line formatted summary output with author attribution (Yeraballi Heena, USN: 25BTRCL295, JAIN, FET CAMPUS) suitable for Terminal and Serial inspection.
- `kernel/src/research/benchmark.rs`: Central Research Benchmark Coordinator:
  - Replicates all 3 experiments from the paper live on the bare-metal kernel:
    - Experiment 1: Processor Affinity (Default, Single CPU 0, Four CPUs 0,2,4,6).
    - Experiment 2: Workload Distribution (1, 2, 4 workers across 4 work units).
    - Experiment 3: CPU-Time Monitoring & Aggregate Utilization (calculates `(100.0 * worker_cpu_time) / elapsed_time`).
  - Global cache: `RESEARCH_RESULTS` singleton for real-time querying.
- `kernel/src/research/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Processor Affinity Bitmask & Bounds (Mask creation, bit manipulation, intersection, string formatting).
  - Test 2: Deterministic CPU Workload Algorithm Integrity (Exact formula and repeatable checksum).
  - Test 3: Affinity-Constrained Task Scheduling (ProcessControlBlock affinity configuration, sched_setaffinity, sched_getaffinity).
  - Test 4: Workload Distribution Multi-Worker Scaling (Multi-worker task execution, 4 work units, checksum consistency).
  - Test 5: CPU-Time Accounting & Aggregate Utilization Metrics (Worker CPU time, elapsed time, aggregate utilization formula).
- `kernel/src/research/mod.rs`: Module exports, banner logging, and `init()` entry point.
- `kernel/src/apps/terminal.rs`: Added interactive shell commands:
  - `research`: Displays full research writeup metadata, algorithm code, and live benchmark tables.
  - `affinity`: Inspects the currently running task's CPU affinity bitmask and allowed logical core list.

### Changed
- `kernel/src/process/pcb.rs`: Added `pub cpu_affinity: u64` and `DEFAULT_CPU_AFFINITY` (`0xFFFF`) to `ProcessControlBlock`, with `set_cpu_affinity`, `get_cpu_affinity`, and `is_cpu_allowed` methods.
- `kernel/src/scheduler/mod.rs`:
  - Updated PIT timer interrupt handler `on_timer_tick` to charge `ticks_used += 1` to `self.current_pid` on each tick (providing accurate per-process CPU-time telemetry).
  - Added `sched_setaffinity` and `sched_getaffinity` APIs.
- `kernel/src/main.rs`: Registered `pub mod research;`, initialized research subsystem via `research::init();`, and executed `research::tests::run_research_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 33 — Full Integration (2026-11-16)

### Added
- `kernel/src/integration/subsystems.rs`: Central Subsystem Catalog & Health Registry:
  - `SubsystemId`: Enumerated identifiers for all 32 subsystems (Phases 1..=32) with phase mapping, category classification (`Core`, `Memory`, `Execution`, `Hardware`, `StorageFs`, `GraphicsUi`, `Connectivity`, `SecurityDebug`), and dependency resolution graphs.
  - `SubsystemState`: Operational state machine (`Operational`, `Degraded`, `Initializing`, `Uninitialized`, `Failed`).
  - `SubsystemInfo`: Comprehensive metadata descriptor with description, dependency checks, and health predicates.
  - `SubsystemRegistry`: Central catalog tracking all 32 subsystems, evaluating live hardware and memory data structures, verifying 100% operational readiness (32/32), and enforcing transitive dependency constraints.
- `kernel/src/integration/matrix.rs`: Cross-Subsystem Integration Matrix & Pipeline Contracts:
  - `PipelineId`: 8 core multi-subsystem pipeline contracts:
    - `Memory`: Physical Memory Manager (PMM) -> Virtual Memory (VMM) -> Kernel Dynamic Heap.
    - `StorageFs`: ATA / IDE Block Storage -> VFS -> Directory Inodes & File Handles.
    - `InputGui`: Hardware Input -> Event Queue -> Window Manager -> Compositor -> GUI Widgets.
    - `HardwareDriver`: Device Registry -> PCI -> USB -> GPU -> Audio -> RTC -> ACPI.
    - `ProcessSync`: Process Control Blocks (PCB) -> Multitasking Scheduler -> Synchronization Primitives.
    - `Network`: Network Device Driver -> Ethernet -> IPv4 -> UDP/TCP -> Socket API.
    - `SecuritySyscall`: Ring 3 Trampoline -> UserPtr Validation -> Capability Enforcement -> Security Audit Trail.
    - `Reliability`: Diagnostic Logger -> Log Ring Buffer -> Exception Decoding -> Stack Unwinder.
  - `IntegrationMatrix::verify_all()`: Validates all 8 cross-subsystem pipeline contracts in real time.
- `kernel/src/integration/health.rs`: Global Kernel Health & Telemetry Collector:
  - `SystemHealthReport`: Gathers live system metrics across memory (usable RAM, free frames, heap bytes), multitasking (task counts, context switches, preemptions, yields), hardware (drivers, PCI devices, USB devices, ACPI tables), filesystem (storage devices, root directory entries), networking (active interfaces, IP, socket tables), security (audit events, credential contexts), and time (monotonic uptime, ISO-8601 wall clock).
  - Emits formatted diagnostics to serial log (`log_summary`).
- `kernel/src/integration/manager.rs`: Central Integration Manager & Diagnostic Coordinator:
  - `IntegrationManager`: Singleton coordinator (`INTEGRATION_MANAGER`) integrating subsystem registry, pipeline verification matrix, and real-time health telemetry.
  - Global query helpers: `with_integration`, `with_integration_mut`, `is_integration_initialized`.
- `kernel/src/integration/suite.rs`: Comprehensive 5-stage regression verification test suite:
  - Test 1: Core Architecture & Memory Health Pipeline (GDT, TSS, IDT, PIC, PIT, PMM frame allocation/free, 4-level paging, heap Box/Vec dynamic allocation).
  - Test 2: Hardware & Driver Subsystem Matrix Integration (Device Manager, PCI bus scan, ATA storage, USB controllers, GPU manager, Audio synthesizer, CMOS RTC uptime, ACPI power tables).
  - Test 3: Process, Scheduling & Synchronization Mesh (PCB table, scheduler context switches/yields, Spinlock, RwLock, CountingSemaphore, Barrier generational cycles).
  - Test 4: Storage, Filesystem & GUI Stack Pipeline (VFS directory and file read/write roundtrip, input event queue, Window Manager and Compositor active state, GUI label widget).
  - Test 5: Security, System Calls & Reliability Telemetry (UserPtr rejection of kernel space and overflow, CapabilitySet permissions and one-way revocation, security audit log recording, KernelErrorCode POSIX errno mapping, bare-metal stack unwinding [3 frames], memory hex dumper).
- `kernel/src/integration/mod.rs`: Full System Integration Layer entry points and re-exports.

### Changed
- `kernel/src/scheduler/mod.rs`: Added `current_pid(&self)` method and top-level helper `scheduler::current_pid()`.
- `kernel/src/process/manager.rs`: Added `current_pid(&self)` accessor method.
- `kernel/src/security/manager.rs`: Added `contexts_count(&self)` accessor method.
- `kernel/src/main.rs`: Registered `pub mod integration;`, initialized integration subsystem via `integration::init();`, and executed `integration::suite::run_integration_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 32 — Debugging + Reliability (2026-11-16)

### Added
- `kernel/src/debug/logger.rs`: Structured kernel logging engine:
  - 6 logging severity levels: `Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`.
  - Dynamic severity threshold filtering (`set_max_level`, `get_max_level`, `is_level_enabled`).
  - Dual-sink dispatch: VGA Text Console (with level color mapping) and COM1 Serial UART (bracketed tags).
  - Diagnostic macros: `klog_trace!`, `klog_debug!`, `klog_warn!`, `klog_error!`, `klog_fatal!`.
- `kernel/src/debug/ring_buffer.rs`: In-memory circular diagnostic log buffer:
  - `LogEntry` structure recording sequence number, timer ticks, severity level, target subsystem, and formatted message.
  - 256-entry capacity ring buffer (`LOG_RING_BUFFER_CAPACITY`) with automatic oldest-entry eviction and `recent(count)` extraction.
- `kernel/src/debug/errors.rs`: Unified kernel error taxonomy & POSIX errno translation:
  - `KernelErrorCode`: 22 standardized error codes matching standard POSIX values (`EPERM`, `ENOENT`, `ESRCH`, `EINTR`, `EIO`, `ENXIO`, `EBADF`, `EAGAIN`, `ENOMEM`, `EACCES`, `EFAULT`, `EBUSY`, `EEXIST`, `ENOTDIR`, `EISDIR`, `EINVAL`, `EMFILE`, `ENOSPC`, `EPIPE`, `EOPNOTSUPP`, `ETIMEDOUT`, `ECONNREFUSED`).
  - `KernelErrorCategory`: Subsystem categorization (`Hardware`, `Memory`, `FileSystem`, `Network`, `Process`, `Security`, `Device`, `Generic`).
  - Bidirectional mapping: `as_errno()`, `from_errno()`, `description()`, and `category()`.
- `kernel/src/debug/exception.rs`: Architectural x86_64 CPU exception fault decoders:
  - `PageFaultError`: Binary bitmask decoder for Vector 14 (`#PF`), extracting Present, Write, User Mode, Reserved Write, Instruction Fetch (NX), and Protection Key flags.
  - `GeneralProtectionError`: Segment selector decoder for Vector 13 (`#GP`), identifying External events, IDT references, GDT/LDT Table Indicators, and descriptor index.
  - `exception_name`: Human-readable architectural exception name lookup.
- `kernel/src/debug/panic.rs`: Stack frame unwinding and diagnostic memory dumping:
  - `read_rbp` and `read_rsp` register extraction routines.
  - `unwind_stack`: Dual-path stack unwinding engine walking the `RBP` frame-pointer chain with fallback call return address scanning up to 2 KiB above `RSP`.
  - `format_hex_dump`: Memory hex dumper with address prefixes, two 8-byte columns, and ASCII character bars.
- `kernel/src/debug/manager.rs`: Central Debug Coordinator (`DebugManager`, `DEBUG_MANAGER`):
  - Integrates in-memory ring buffer, CPU/memory fault counters, panic counts, and global accessors (`with_debug`, `with_debug_mut`).
- `kernel/src/debug/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Structured logging severity threshold filtering & macro emissions.
  - Test 2: In-memory circular log ring buffer capacity wrap-around & recent history retrieval.
  - Test 3: Unified kernel error codes, subsystem categories & POSIX errno roundtrip.
  - Test 4: Architectural CPU exception decoding (#PF bitmask flags & #GP descriptor selectors).
  - Test 5: Stack frame unwinding (depth=3 verified), register readouts, and memory hex dump formatting.

### Changed
- `.cargo/config.toml`: Added `"-C", "force-frame-pointers=yes"` to guarantee architectural frame-pointer generation across all functions for reliable backtraces.
- `kernel/src/main.rs`: Registered `pub mod debug;`, initialized debugging subsystem via `debug::init();`, and executed `debug::tests::run_debug_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 31 — Kernel Synchronization (2026-11-16)

### Added
- `kernel/src/sync/spinlock.rs`: Spinlock synchronization and hardware interrupt critical section management:
  - `without_interrupts`: Disables hardware interrupts, executes closure, and restores previous `IF` state in RFLAGS.
  - `push_and_disable_interrupts` / `restore_interrupts`: Direct RFLAGS inspection and `cli`/`sti` handling.
  - `IrqSpinlock<T>` & `IrqSpinlockGuard<T>`: Interrupt-safe spinlock protecting against concurrent execution from interrupt service routines (ISRs), with non-blocking `try_lock()` and RAII automatic release.
  - `Spinlock<T>` & `SpinlockGuard<T>`: Lightweight spinlock for non-interrupt contexts.
- `kernel/src/sync/rwlock.rs`: Reader-Writer Lock (`RwLock<T>`):
  - Supports concurrent shared readers (`read()`, `try_read()`, `RwLockReadGuard`) and exclusive writer access (`write()`, `try_write()`, `RwLockWriteGuard`).
  - Active reader counting and writer exclusion state machine.
- `kernel/src/sync/mutex.rs`: Cooperative Sleeping Kernel Mutex (`KernelMutex<T>`):
  - Mutex primitive integrating with the task scheduler to cooperatively yield CPU timeslices on contention via `crate::scheduler::yield_now()`, avoiding CPU cycle burning.
  - Non-blocking `try_lock()`, lock status query (`is_locked()`), and RAII `MutexGuard<T>`.
- `kernel/src/sync/semaphore.rs`: Dijkstra Counting Semaphore (`CountingSemaphore`):
  - Thread-safe permit-based resource synchronization and bounded concurrency limits.
  - `acquire()` with cooperative yielding on zero permits, non-blocking `try_acquire()`, bounded `release()`, and `available_permits()`.
- `kernel/src/sync/wait_queue.rs`: Process Wait Queue (`WaitQueue`):
  - FIFO queue for managing processes blocked on events or locks.
  - `wait_pid()`, `wait_current()`, `wake_one()`, and `wake_all()` with seamless integration into `PROCESS_MANAGER` and `SCHEDULER`.
- `kernel/src/sync/condvar.rs`: Condition Variable (`Condvar`):
  - Enables tasks to wait for a shared predicate condition while atomically releasing an associated mutex.
  - `wait()`, `notify_one()`, `notify_all()`, and `waiter_count()`.
- `kernel/src/sync/once.rs`: Thread-safe one-time initialization primitives:
  - `Once`: Multi-state flag (`INCOMPLETE`, `RUNNING`, `COMPLETE`) executing an initialization closure exactly once across all cores.
  - `OnceCell<T>`: Thread-safe lazy initialization cell with `set()`, `get()`, and `get_or_init()`.
- `kernel/src/sync/barrier.rs`: Task rendezvous barrier (`Barrier`, `BarrierWaitResult`):
  - Coordinates `N` threads meeting at an execution checkpoint with generational tracking, automatic reset, and single leader election.
- `kernel/src/sync/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Atomic CAS, arithmetic ordering, Spinlock & IrqSpinlock mutual exclusion, and interrupt masking.
  - Test 2: Reader-Writer Lock (RwLock) concurrent readers (4 active readers) & exclusive writer isolation.
  - Test 3: Kernel Sleeping Mutex (KernelMutex) mutual exclusion, try_lock, and RAII lifecycle.
  - Test 4: Counting Semaphore permit consumption, exhaustion, capacity bounds, and release cycling.
  - Test 5: Task Wait Queue FIFO ordering, Condition Variable notifications, OnceCell lazy initialization, and Barrier rendezvous.

### Changed
- `kernel/src/scheduler/mod.rs`: Added `wake_task(pid)` and `block_current()` scheduler primitives to support blocking task queues and synchronization primitives.
- `kernel/src/main.rs`: Replaced legacy single-file `kernel/src/sync.rs` with the modular `kernel/src/sync/` subsystem, initialized via `sync::init();`, and executed `sync::tests::run_sync_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 30 — Security + Isolation (2026-11-16)

### Added
- `kernel/src/security/types.rs`: Core security primitives, capabilities, quotas, and contexts:
  - `Capability`: 14 fine-grained system privileges (`ReadStorage`, `WriteStorage`, `NetworkAccess`, `HardwareIo`, `ManageProcesses`, `SystemAdmin`, `AudioAccess`, `GpuAccess`, `PowerManagement`, `IpcCommunication`, `AuditLog`, `RawSockets`, `KernelModules`, `SetResourceLimits`).
  - `CapabilitySet`: 32-bit capability mask with bitwise operations, presets (`NONE`, `ALL`, `STANDARD_USER`, `SANDBOX_STRICT`), subset tests, intersection, and containment checks.
  - `PointerViolation`: Structured classification for pointer faults (`NullPointer`, `UnderflowLowMemory`, `KernelAddressViolation`, `ArithmeticOverflow`, `ZeroLength`, `Unaligned`, `PageNotPresent`, `SupervisorPage`, `WriteOnReadOnly`).
  - `ResourceLimitType` & `ResourceLimits`: Process resource quotas (maximum memory bytes, maximum open files, maximum child processes, maximum CPU execution time ms) with dynamic accounting and limit checking.
  - `SecurityError`: Strongly-typed error codes (`PermissionDenied`, `MissingCapability`, `ResourceLimitExceeded`, `InvalidPointer`, `SandboxViolation`, `ProcessNotFound`, `PrivilegeElevationForbidden`).
  - `SecurityContext`: Process credentials and security boundary (`pid`, `uid`, `gid`, `capabilities`, `bounding_set`, `limits`, `is_sandboxed`, `name`) with permanent one-way capability dropping and anti-elevation protection.
- `kernel/src/security/pointer.rs`: User-space memory pointer and buffer security validator:
  - Boundary definitions: `USER_SPACE_MIN` (0x10000, protecting low 64 KiB BIOS/IVT/BDA), `USER_SPACE_MAX` (0x0000_7FFF_FFFF_FFFF, 128 TiB canonical user limit), `KERNEL_SPACE_MIN` (0xFFFF_8000_0000_0000).
  - `validate_user_address`: Hardware-enforced bounds check, arithmetic overflow check, and 4-level MMU page table permission validation (`PRESENT`, `USER_ACCESSIBLE`, `WRITABLE`).
  - `UserPtr<T>`: Strongly-typed user pointer abstraction with alignment validation and safe `read()` / `write()` operations.
  - `UserSlice<T>`: Contiguous user memory slice with bounds verification and safe `copy_to_slice()` / `copy_from_slice()` operations.
  - `copy_str_from_user`: Safe extraction of null-terminated UTF-8 strings from user memory.
- `kernel/src/security/audit.rs`: Structured security audit trail subsystem:
  - `AuditSeverity`: `Info`, `Warning`, `Denial`, `Fault`, `Critical`.
  - `AuditEventKind`: Detailed categorization (`PointerFault`, `CapabilityDenied`, `ResourceExceeded`, `PrivilegeDropped`, `SandboxEntered`, `UnauthorizedAccessAttempt`).
  - `AuditEvent`: Monotonically sequenced, PIT-timestamped audit record.
  - `AuditLog`: 128-entry bounded in-memory ring buffer with total denial and fault counters.
- `kernel/src/security/sandbox.rs`: Multi-tier process isolation and containment policies:
  - `SandboxPolicy`: Configurable isolation policy with presets (`STRICT`, `READ_ONLY_FS`, `COMPUTE_ONLY`).
  - Irrevocable sandbox containment enforcement: drops all unpermitted capabilities, clears process management and hardware I/O, and enforces memory/file quotas.
- `kernel/src/security/manager.rs`: Central Security Coordinator (`SecurityManager`, `SECURITY_MANAGER`):
  - Per-PID `SecurityContext` table, capability checks, privilege dropping, resource charge/release accounting, audit trail integration, and safe global accessors (`with_security`, `with_security_mut`).
- `kernel/src/security/mod.rs`: Subsystem module exports, `security::init()` (bootstrap PID 0 root supervisor registration), and `is_security_initialized()`.
- `kernel/src/security/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: User pointer validation & boundary protection (NULL, low BIOS, arithmetic overflow, canonical limits, supervisor page, unmapped page, `UserPtr` read, unaligned rejection, `UserSlice` copy).
  - Test 2: Capability-based access control & privilege revocation (standard capabilities, privileged denial, one-way capability dropping, anti-elevation defense).
  - Test 3: Process resource accounting & quota enforcement (memory quota, open file descriptor limit, child process count, CPU time budget).
  - Test 4: Strict sandbox & multi-tier process isolation (sandbox flag, hardware I/O denial, disk mutation denial, process spawn denial, IPC retention, read-only filesystem policy).
  - Test 5: Security audit logging & fault containment (audit trail active, denials tracking, fault tracking, zero kernel panic).

### Changed
- `kernel/src/main.rs`: Registered `pub mod security;`, initialized security subsystem via `security::init();`, and executed `security::tests::run_security_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 29 — ACPI + Power Management (2026-11-16)

### Added
- `kernel/src/acpi/types.rs`: Core ACPI primitives, signatures, and power structures:
  - `AcpiSignature`: 4-character table signature representation (`RSDT`, `XSDT`, `FADT`/`FACP`, `DSDT`, `MADT`/`APIC`, `HPET`, `MCFG`, `SSDT`, `FACS`).
  - `GenericAddress`: ACPI Generic Address Structure (GAS) with address space identification (System Memory, System I/O), bit offsets, and access sizes.
  - `SleepState`: Standard ACPI system power states: S0 (Working), S1 (Standby), S2 (Deep Standby), S3 (Suspend-to-RAM), S4 (Hibernate), and S5 (Soft Off / Shutdown).
  - `BatteryStatus`: Battery telemetry reporting presence, charging state, and charge percentage.
  - `AcpiError`: Error taxonomy covering RSDP absence, checksum verification failures, malformed headers, and hardware faults.
- `kernel/src/acpi/sdt.rs`: Binary structure definitions for RSDP and SDT headers:
  - `calculate_checksum`: 8-bit byte-sum validation routine.
  - `Rsdp`: 36-byte Root System Description Pointer structure supporting ACPI 1.0 (20-byte) and ACPI 2.0+ (36-byte extended) checksums, OEM ID strings, and XSDT/RSDT root table address resolution.
  - `SdtHeader`: Standard 36-byte ACPI table header with `verify_checksum_at` for physical memory table validation.
- `kernel/src/acpi/rsdp.rs`: Physical memory scanning engine for RSDP discovery:
  - Scans Extended BIOS Data Area (EBDA) via base segment pointer at physical address 0x040E.
  - Scans BIOS Read-Only Memory space [0x000E0000 .. 0x000FFFFF] on 16-byte aligned boundaries.
  - Validates "RSD PTR " magic signature and 8-bit checksum.
- `kernel/src/acpi/tables.rs`: Root table and specialized description table parsers:
  - `AcpiTables`, `TableEntry`: Multi-table dispatcher parsing 64-bit pointers (XSDT) and 32-bit pointers (RSDT) using unaligned-safe memory reads, checksum verification, and table lookup by signature.
  - `Fadt`: Fixed ACPI Description Table parser decoding DSDT address, SMI command port, ACPI enable/disable bytes, PM1a/PM1b control blocks, PM1a event block, PM timer block, century register, and hardware reset register structure.
  - `Madt`: Multiple APIC Description Table parser decoding Local APIC address, CPU core count, I/O APIC discovery, and Interrupt Source Overrides.
- `kernel/src/acpi/power.rs`: System power management and reset controllers:
  - `AcpiPowerManager`:
    - Transitions hardware into ACPI mode by writing `ACPI_ENABLE` to the SMI command port.
    - Multi-tier system reboot: ACPI reset register, 8042 PS/2 keyboard controller pulse (Port 0x64 <- 0xFE), and processor triple-fault fallback.
    - System shutdown (S5 Soft Off) via PM1 control registers and hypervisor shutdown ports (VirtualBox 0x4004, QEMU 0x0604, Bochs 0xB004).
- `kernel/src/acpi/manager.rs`: Central ACPI Subsystem Manager (`AcpiManager`, `ACPI_MANAGER`):
  - Coordinates table discovery, power state management, and exposes public control APIs (`reboot()`, `power_off()`, `with_acpi()`).
- `kernel/src/acpi/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: RSDP discovery & checksum validation.
  - Test 2: Root table (RSDT / XSDT) parsing & table signature enumeration.
  - Test 3: Fixed ACPI Description Table (FADT) & power management block decoding.
  - Test 4: Multiple APIC Description Table (MADT) & APIC topology discovery.
  - Test 5: ACPI Power Management state machine & reset register capabilities.

### Changed
- `kernel/src/drivers/wrappers.rs`: Implemented `AcpiDriver` on `AcpiFoundationDevice`, dynamically reporting `HardwareSupportStatus::Supported` when `acpi::is_acpi_initialized()` is active.
- `kernel/src/main.rs`: Registered `pub mod acpi;`, initialized ACPI subsystem via `acpi::init();`, and executed `acpi::tests::run_acpi_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 28 — RTC + Time (2026-11-15)

### Added
- `kernel/src/time/types.rs`: Gregorian calendar types, formatting, and duration primitives:
  - `Weekday`: Enum representing Sunday through Saturday with Sakamoto's day-of-week evaluation and full/abbreviated string conversion.
  - `Month`: Enum representing January through December.
  - `DateTime`: Complete Gregorian calendar representation (year, month, day, hour, minute, second):
    - Gregorian leap year evaluation (`is_leap_year`).
    - Dynamic days-in-month calculation (`days_in_month`).
    - ISO-8601 standard string serialization (`YYYY-MM-DDTHH:MM:SSZ`).
    - Human-readable timestamp formatting (`YYYY-MM-DD HH:MM:SS`).
    - Bidirectional conversion between `DateTime` and 64-bit Unix Epoch timestamps.
  - `Duration`: Microseconds, milliseconds, and seconds duration arithmetic with scaling helpers.
  - `TimeError`: Hardware and calendar error codes (`RtcNotReady`, `InvalidDateTime`, `BatteryDepleted`, `TimerNotFound`, `TimerLimitReached`).
- `kernel/src/time/cmos.rs`: Motorola MC146818 CMOS Real-Time Clock (RTC) hardware driver:
  - Hardware port I/O via ports 0x70 (index/NMI) and 0x71 (data).
  - Status Register A Update-In-Progress (`UIP`) polling with spinlock timeout protection.
  - Debounced consecutive double-read register reading to prevent mid-tick roll-overs.
  - Status Register D battery health monitoring.
  - Status Register B Binary vs BCD decoding and 12-hour AM/PM vs 24-hour military mode conversion.
  - Century register decoding and full 4-digit Gregorian year synthesis.
- `kernel/src/time/clock.rs`: Monotonic uptime and synchronized wall-clock engines:
  - `MonotonicClock`: Strictly non-decreasing hardware tick counter derived from 8254 PIT (100 Hz = 10 ms granularity) with millisecond and microsecond scaling.
  - `WallClock`: Real-world civil time anchor synchronized with CMOS RTC at boot and updated with monotonic elapsed time.
- `kernel/src/time/timer.rs`: Central software timer service (`TimerService`, `TimerEntry`, `TimerId`, `TimerMode`):
  - Support for `OneShot` and `Periodic` software timers.
  - Monotonic deadline checking, automated periodic interval rescheduling, and timer cancellation.
- `kernel/src/time/manager.rs`: Central Time Subsystem Manager (`TimeManager`, `TIME_MANAGER`):
  - Initialization of CMOS RTC, wall-clock synchronization, and global convenience functions (`now()`, `timestamp()`, `uptime_ms()`, `format_now_iso8601()`, `with_time_mut`).
- `kernel/src/time/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: CMOS RTC hardware detection, status registers & BCD decoding.
  - Test 2: Gregorian calendar arithmetic, leap years, weekdays & ISO-8601 formatting.
  - Test 3: Unix Epoch timestamp bidirectional conversion roundtrip.
  - Test 4: Monotonic uptime clock tracking & non-decreasing guarantee.
  - Test 5: Software timer engine (One-shot, periodic & cancellation).

### Changed
- `kernel/src/drivers/wrappers.rs`: Implemented `CmosRtcDevice` conforming to `RtcDriver` and `Driver` traits, reporting `HardwareSupportStatus::Supported` when `time::is_time_initialized()` is active.
- `kernel/src/drivers/manager.rs`: Registered `CmosRtcDevice` into the central operating system device registry.
- `kernel/src/main.rs`: Registered `pub mod time;`, initialized time subsystem via `time::init();`, and executed `time::tests::run_time_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 27 — Audio (2026-11-15)

### Added
- `kernel/src/audio/types.rs`: Core audio types, stream descriptors, and capability definitions:
  - `SampleFormat`: `U8` (8-bit unsigned), `S16Le` (16-bit signed LE), `S24Le` (24-bit in 32-bit container), `S32Le` (32-bit signed), `F32Le` (32-bit IEEE float).
  - `AudioChannels`: `Mono` (1), `Stereo` (2), `Surround51` (6), `Surround71` (8).
  - `SampleRate`: Standard frequencies (8000 Hz, 11025 Hz, 22050 Hz, 44100 Hz, 48000 Hz, 96000 Hz).
  - `AudioFormat`: Stream format descriptor with frame size, byte rate, millisecond duration calculation, and presets (`CD_QUALITY`, `DVD_QUALITY`, `VOICE_MONO`).
  - `AudioState`: Playback state machine (`Stopped`, `Playing`, `Paused`).
  - `AudioCapabilities`: Controller capabilities (channel limits, bit depths, sample rates, mixer support, DMA buffer capacity).
  - `AudioError`: Unified error taxonomy for device, buffer, DMA, and parameter conditions.
- `kernel/src/audio/buffer.rs`: Dynamically sized PCM audio buffer (`PcmBuffer`):
  - Dynamic sample storage, frame indexing, duration querying, and byte slice conversions.
  - S16Le sample access (`push_sample_s16`, `sample_s16`), silence initialization.
  - In-place software volume attenuation (`apply_volume`).
  - Peak amplitude analysis (`peak_amplitude_s16`).
  - Multi-buffer mixing with saturation limiting (`mix_s16`).
- `kernel/src/audio/synth.rs`: Mathematical waveform synthesizer & sound presets:
  - `Waveform`: `Sine`, `Square`, `Sawtooth`, `Triangle`, and pseudo-random `Noise`.
  - High-precision Bhaskara I polynomial sine approximation in pure `#![no_std]` without external math libraries.
  - Musical note frequency tables (`Notes::C4` through `Notes::A5 = 880 Hz`).
  - System sound presets: `StartupChime` (3-note major arpeggio), `AlertBeep` (880 Hz sine), `NotifyTone` (two-tone chime), `ErrorBuzz` (150 Hz square).
- `kernel/src/audio/mixer.rs`: Multi-track software audio mixer (`AudioMixer`, `MixerTrack`):
  - Concurrent multi-track playback, dynamic track allocation, volume control, and loop flags.
  - 32-bit accumulation and saturation clamping (`i16::clamp`) preventing audio clipping distortion.
  - Master volume scaling and global mute state.
- `kernel/src/audio/speaker.rs`: Motherboard PC Speaker hardware driver (`PcSpeaker`):
  - 8254 PIT Channel 2 square wave frequency modulation (ports `0x42`/`0x43`) and system control port B (`0x61`) gate manipulation for hardware beeps and alerts.
- `kernel/src/audio/traits.rs`: `AudioDriver` uniform hardware abstraction trait.
- `kernel/src/audio/drivers/`: Concrete audio drivers:
  - `virtual_audio.rs`: `VirtualAudioDriver` in-memory loopback sink with 64 KiB ring buffer, peak tracking, and deterministic test telemetry.
  - `hda.rs`: `IntelHdaDriver` for Intel High Definition Audio / Azalia controllers (PCI Class 0x04, Subclass 0x03).
  - `ac97.rs`: `Ac97Driver` for AC'97 2.3 audio controllers (PCI Class 0x04, Subclass 0x01).
  - `mod.rs`: Driver exports.
- `kernel/src/audio/manager.rs`: Central Audio Subsystem Manager (`AudioManager`, `AUDIO_MANAGER`):
  - Automatic PCI multimedia controller detection via `PCI_MANAGER` and driver binding.
  - High-level playback APIs (`play_tone`, `play_preset`, `play_buffer`, `hardware_beep`).
  - Mixer buffer flushing to active hardware or virtual driver.
- `kernel/src/audio/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Audio controller detection & driver abstraction.
  - Test 2: PCM buffer abstraction & stream format mathematics.
  - Test 3: Mathematical waveform synthesis (Sine, Square, Sawtooth, Triangle, Noise) & presets.
  - Test 4: Software audio mixer, multi-track blending & saturation clamping.
  - Test 5: Playback stream pipeline, DMA / FIFO write operations & state machine.

### Changed
- `kernel/src/drivers/device.rs`: Added `Audio` category to `DeviceCategory`.
- `kernel/src/drivers/wrappers.rs`: Implemented `AudioFoundationDevice` dynamically reporting `HardwareSupportStatus::Supported` when `audio::is_audio_initialized()` is active.
- `kernel/src/drivers/manager.rs`: Registered `AudioFoundationDevice` into the central device registry.
- `kernel/src/main.rs`: Registered `pub mod audio;`, initialized audio subsystem via `audio::init();`, and executed `audio::tests::run_audio_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

## Phase 26 — GPU Foundation (2026-11-15)

### Added
- `kernel/src/gpu/types.rs`: Core GPU primitives, display modes, and capabilities:
  - `PixelFormat`: `Bgra8888` (32-bit B-G-R-A), `Rgba8888` (32-bit R-G-B-A), `Bgr888` (24-bit B-G-R), and `Rgb565` (16-bit packed 5-6-5).
  - `DisplayMode`: Predefined video mode configurations:
    - 640x480 @ 32 bpp (4:3 VGA)
    - 800x600 @ 32 bpp (4:3 SVGA)
    - 1024x768 @ 32 bpp (4:3 XGA standard baseline)
    - 1280x720 @ 32 bpp (16:9 HD)
    - 1920x1080 @ 32 bpp (16:9 Full HD)
  - `GpuVendor`: Vendor recognition for Intel, AMD, NVIDIA, VirtualBox (0x80EE), QEMU (0x1234), VMware (0x15AD), and generic VBE adapters.
  - `GpuCapabilities`: Hardware acceleration reporting (2D engine, hardware cursor, double buffering, command queue, VRAM capacity).
  - `FramebufferInfo`: Physical LFB base address, dimensions, pitch, bpp, and pixel format layout.
  - `GpuError`: Comprehensive error taxonomy for unsupported modes, surface faults, queue capacity, and device initialization failures.
- `kernel/src/gpu/device.rs`: PCI display adapter representation:
  - `GpuDevice`: PCI vendor ID, device ID, subsystem identity, VRAM sizing, and BAR address resolution.
  - LFB physical address decoding via BAR0/BAR1 MMIO mapping.
- `kernel/src/gpu/surface.rs`: Surface allocation and lifecycle management:
  - `SurfaceId`: Unique identifier differentiating scanout LFB (Surface 0) from offscreen render targets.
  - `GpuSurface`: Pixel buffer metadata, dimension tracking, stride/pitch alignment, boundary-checked pixel access, and block memory rectangle filling.
  - `SurfaceManager`: Dynamic creation, lookup, mutation, and destruction of GPU surfaces.
- `kernel/src/gpu/command.rs`: 2D GPU Command Stream architecture:
  - `GpuCommand`: Asynchronous graphics commands (`Nop`, `Clear`, `FillRect`, `Blit`, `DrawLine`, `Present`, `Fence`).
  - `GpuCommandQueue`: FIFO command ring buffer with overflow protection and monotonic fence tracking.
- `kernel/src/gpu/traits.rs`: `GpuDriver` hardware abstraction trait:
  - Unified driver interface covering initialization, capability queries, supported mode enumeration, mode setting, surface management, 2D command submission, and fence synchronization.
- `kernel/src/gpu/engine.rs`: High-performance 2D rasterization engine:
  - `GpuEngine`: Execution pipeline for 2D commands, boundary-clipping rectangles, block memory blits, and Bresenham's integer line rasterization algorithm.
- `kernel/src/gpu/drivers/`: Concrete GPU driver implementations:
  - `vbox.rs`: VirtualBox PCI Graphics Adapter driver (`0x80EE:0xBEEF` VBE/BGA) with Dispi I/O port control and LFB integration.
  - `qemu.rs`: QEMU Standard VGA PCI adapter driver (`0x1234:0x1111`) using Bochs VBE index/data register programming.
  - `vmware.rs`: VMware SVGA II display driver (`0x15AD:0x0405`) with SVGA FIFO register protocol.
  - `software.rs`: Architectural software rasterizer fallback driver.
- `kernel/src/gpu/manager.rs`: Central GPU Manager (`GpuManager`, `GPU_MANAGER`):
  - Dynamic discovery of PCI display controllers (Class 0x03, Subclasses 0x00 and 0x80).
  - Automated driver selection and binding based on PCI vendor and device ID.
  - Safe global access via `with_gpu_mut` closure helper and IRQ-safe spinlocks.
- `kernel/src/gpu/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: PCI Display Controller identification (Vendor/Device ID, Class 0x03, VRAM & LFB).
  - Test 2: GPU Device abstraction & hardware capability query.
  - Test 3: Display Mode configuration & Framebuffer baseline (1024x768x32).
  - Test 4: GPU Surface management & offscreen render targets.
  - Test 5: 2D Command Stream execution, Bresenham's line rasterization, and fence synchronization.

### Changed
- `kernel/src/drivers/wrappers.rs`: Added `GpuFoundationDevice` dynamically reporting `HardwareSupportStatus::Supported` when `gpu::is_gpu_initialized()` is true.
- `kernel/src/drivers/manager.rs`: Registered `GpuFoundationDevice` into the central operating system device registry.
- `kernel/src/main.rs`: Registered `pub mod gpu;`, initialized GPU subsystem via `gpu::init();`, and executed `gpu::tests::run_gpu_tests();`, verified live in VirtualBox VM `IntegrateOS_D`.

## Phase 25 — USB (2026-11-15)

### Added
- `kernel/src/usb/types.rs`: Standard USB specification primitives and enumerations:
  - `UsbSpeed`: Low-Speed (1.5 Mbps), Full-Speed (12 Mbps), High-Speed (480 Mbps), SuperSpeed (5 Gbps), and SuperSpeedPlus (10 Gbps).
  - `UsbTransferType`: Control, Isochronous, Bulk, and Interrupt.
  - Transfer directions (`HostToDevice`, `DeviceToHost`), request types (`Standard`, `Class`, `Vendor`), and recipients (`Device`, `Interface`, `Endpoint`, `Other`).
  - Standard USB request constants (Table 9-4: `GET_STATUS`, `CLEAR_FEATURE`, `SET_ADDRESS`, `GET_DESCRIPTOR`, `SET_CONFIGURATION`, etc.).
  - Standard descriptor constants (Table 9-5: `DEVICE`, `CONFIGURATION`, `STRING`, `INTERFACE`, `ENDPOINT`, `HID`, `HUB`).
  - Device class identifiers (`AUDIO`, `CDC`, `HID`, `MASS_STORAGE`, `HUB`, `SMART_CARD`, `VIDEO`, `WIRELESS`).
  - HID and Mass Storage BOT error definitions and request codes.
- `kernel/src/usb/setup.rs`: 8-byte standard USB Setup packet builder and parser:
  - `UsbSetupPacket` struct (`bmRequestType`, `bRequest`, `wValue`, `wIndex`, `wLength`).
  - Helper constructors: `get_descriptor`, `set_address`, `get_configuration`, `set_configuration`, `clear_feature_endpoint`, `hid_set_protocol`, `hid_set_idle`, `hid_get_report`, `msc_bot_reset`, `msc_get_max_lun`.
- `kernel/src/usb/descriptors.rs`: Binary-safe USB descriptor representations:
  - `UsbDeviceDescriptor` (18 bytes): standard device identity parsing supporting full and partial descriptor responses.
  - `UsbConfigurationDescriptor` (9 bytes): power consumption, attributes, and total length calculation.
  - `UsbInterfaceDescriptor` (9 bytes): interface class, subclass, and protocol extraction.
  - `UsbEndpointDescriptor` (7 bytes): endpoint address, transfer direction, and transfer type parsing.
  - `UsbHidDescriptor` (9 bytes): HID specification version and report descriptor length.
  - `parse_string_descriptor`: UTF-16LE string descriptor decoding to ASCII / Rust `String`.
  - `parse_configuration_tree`: Full recursive configuration parser separating interfaces, endpoints, and HID descriptors.
- `kernel/src/usb/controller.rs`: `UsbHostController` trait and `UsbControllerType`:
  - Unified hardware abstraction covering UHCI, OHCI, EHCI, xHCI, and virtual host controllers.
  - Pipe operations: `control_transfer`, `bulk_transfer`, `interrupt_transfer`, port polling, and port resetting.
- `kernel/src/usb/host/`: Concrete USB Host Controller implementations:
  - `xhci.rs`: USB 3.0 Extensible Host Controller Interface capability and operational register sets.
  - `ehci.rs`: USB 2.0 Enhanced Host Controller Interface registers (USBCMD, USBSTS, USBINTR, PORTSC).
  - `uhci.rs`: USB 1.1 Universal Host Controller Interface port I/O registers (USBCMD, USBSTS, PORTSC1/2).
  - `virtual_host.rs`: Fully functional software USB 2.0 Host Controller & Root Hub managing virtual root ports, answering control transfers, serving HID interrupt reports, and executing Bulk-Only Transport SCSI read/write block operations.
- `kernel/src/usb/device.rs`: Comprehensive `UsbDevice` representation:
  - Active peripheral state: bus address, link speed, controller index, port number, descriptors, and parsed configuration tree.
  - Classification helpers: `is_hid`, `is_hid_keyboard`, `is_hid_mouse`, `is_mass_storage`, `is_hub`, and `find_endpoint`.
- `kernel/src/usb/manager.rs`: Central USB Manager (`UsbManager`, `USB_MANAGER`):
  - Dynamic discovery of physical PCI host controllers (UHCI/EHCI/xHCI) and registration of the virtual root hub.
  - 9-step peripheral enumeration state machine (Port reset -> partial device descriptor query -> SET_ADDRESS -> full device descriptor -> configuration tree -> string descriptors -> SET_CONFIGURATION -> driver binding).
  - Automatic class driver binding for HID Keyboards, Mice, and Mass Storage block devices.
  - Safe deadlock-free host controller locking via `with_controller_mut`.
- `kernel/src/usb/hid/`: Human Interface Device subsystem:
  - `usage.rs`: HID Usage Table Page 0x07 (Keyboard/Keypad) to `KeyCode` and ASCII conversion.
  - `keyboard.rs`: `UsbHidKeyboard` decoding 8-byte boot keyboard reports, tracking modifiers, and pushing `KeyEvent`s and ASCII characters to `KEYBOARD_QUEUE`.
  - `mouse.rs`: `UsbHidMouse` decoding 3/4-byte boot mouse reports, tracking cursor coordinates with screen clamping, and pushing `MouseEvent`s to the system input queue.
- `kernel/src/usb/msc/`: USB Mass Storage subsystem:
  - `bot.rs`: Bulk-Only Transport protocol implementing 31-byte Command Block Wrapper (`CBW`) and 13-byte Command Status Wrapper (`CSW`).
  - `scsi.rs`: SCSI Transparent Command Set (SPC/SBC) CDB builders for `INQUIRY (0x12)`, `TEST_UNIT_READY (0x00)`, `READ_CAPACITY_10 (0x25)`, `READ_10 (0x28)`, and `WRITE_10 (0x2A)`.
  - `driver.rs`: `UsbStorageDevice` implementing `StorageDriver` block device trait for 512-byte sector read and write operations.
- `kernel/src/usb/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Host controller discovery & abstraction.
  - Test 2: Standard Setup packet serialization & descriptor parsing.
  - Test 3: USB bus enumeration & peripheral device topology.
  - Test 4: USB HID Boot Keyboard & Mouse drivers with input queue event routing.
  - Test 5: USB Mass Storage Bulk-Only Transport & SCSI block read/write operations.

### Changed
- `kernel/src/drivers/wrappers.rs`: Updated `UsbFoundationDevice::status()` to return `HardwareSupportStatus::Supported` dynamically once `usb::is_usb_initialized()` is true.
- `kernel/src/pci/bar.rs`: Added `mmio_address(&self) -> Option<u64>` and `io_port(&self) -> Option<u16>` helpers to `Bar`.
- `kernel/src/pci/address.rs`: Added `format(&self) -> String` BDF notation alias to `PciAddress`.
- `kernel/src/main.rs`: Registered `pub mod usb;`, initialized USB subsystem with `usb::init();`, and executed `usb::tests::run_usb_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 24 — PCI / PCIe (2026-11-15)

### Added
- `kernel/src/pci/types.rs`: Standard PCI 2.3 / PCIe configuration space specifications and registers:
  - Configuration register offsets: `CONFIG_ADDRESS` (0xCF8), `CONFIG_DATA` (0xCFC), `VENDOR_ID`, `DEVICE_ID`, `COMMAND`, `STATUS`, `REVISION_ID`, `PROG_IF`, `SUBCLASS`, `CLASS_CODE`, `CACHE_LINE_SIZE`, `LATENCY_TIMER`, `HEADER_TYPE`, `BIST`, `BAR0`..`BAR5`, `CAPABILITIES_PTR`, `INTERRUPT_LINE`, `INTERRUPT_PIN`.
  - Bitfield bitmasks: Command register (`IO_SPACE`, `MEMORY_SPACE`, `BUS_MASTER`, `SPECIAL_CYCLES`, `MEM_WRITE_INVALIDATE`, `VGA_PALETTE_SNOOP`, `PARITY_ERROR_RESPONSE`, `SERR_ENABLE`, `FAST_BACK_TO_BACK`, `INTERRUPT_DISABLE`) and Status register (`CAPABILITIES_LIST`, `66MHZ_CAPABLE`, `FAST_BACK_TO_BACK_CAPABLE`, `MASTER_DATA_PARITY_ERROR`, `DEVSEL_TIMING`, `SIGNALED_TARGET_ABORT`, `RECEIVED_TARGET_ABORT`, `RECEIVED_MASTER_ABORT`, `SIGNALED_SYSTEM_ERROR`, `DETECTED_PARITY_ERROR`).
  - `PciHeaderType`: Standard device (`GeneralDevice = 0x00`), PCI-to-PCI Bridge (`PciBridge = 0x01`), and CardBus Bridge (`CardBusBridge = 0x02`).
- `kernel/src/pci/address.rs`: Strongly typed `PciAddress` (Bus 0..255, Device 0..31, Function 0..7):
  - Configuration Mechanism #1 access via x86 Port I/O `0xCF8` and `0xCFC` using `spin_lock` IRQ-safe synchronization.
  - Granular accessor methods: `read_u32`, `write_u32`, `read_u16`, `write_u16`, `read_u8`, `write_u8` with bit-shift offset alignment.
  - BDF formatting helper (`format_bdf` -> `"00:03.0"`).
- `kernel/src/pci/bar.rs`: Base Address Register (BAR) probing, sizing and decoding:
  - `PciBar` enum: `None`, `Io { port, size }`, `Mmio32 { address, size, prefetchable }`, `Mmio64 { address, size, prefetchable }`.
  - Probing algorithm writing all-ones (`0xFFFFFFFF`) to BAR register and reading back size mask with bitwise inversion + 1.
  - 64-bit BAR decoding supporting paired low/high registers with atomic 64-bit address reconstruction.
- `kernel/src/pci/classes.rs`: PCI Base Class, Subclass, and Programming Interface decoding table:
  - Enumeration and human-readable string descriptions for Storage (`IDE`, `AHCI`, `NVMe`, `SCSI`, `RAID`), Network (`Ethernet`, `TokenRing`, `FDDI`, `Wireless`), Display (`VGA`, `XGA`, `3D`), Multimedia (`Video`, `Audio`, `HDA`), Memory, Bridge (`Host`, `ISA`, `PCI-to-PCI`, `CardBus`), Simple Communication (`Serial 16550`, `Parallel`), Base System (`PIC`, `DMA`, `Timer`, `RTC`), Input (`Keyboard`, `Digitizer`, `Mouse`), and Serial Bus (`FireWire`, `ACCESS.bus`, `SSA`, `USB UHCI/OHCI/EHCI/xHCI`).
- `kernel/src/pci/capabilities.rs`: PCI Capability Linked-List Parser:
  - Linked-list traversal starting at configuration offset `0x34` (`CAPABILITIES_PTR`).
  - Decoding capability IDs: `PowerManagement = 0x01`, `Agp = 0x02`, `Vpd = 0x03`, `SlotId = 0x04`, `Msi = 0x05`, `CompactPciHotSwap = 0x06`, `PciX = 0x07`, `HyperTransport = 0x08`, `VendorSpecific = 0x09`, `DebugPort = 0x0A`, `CompactPciCrc = 0x0B`, `HotPlug = 0x0C`, `SubsystemId = 0x0D`, `Agp8x = 0x0E`, `SecureDevice = 0x0F`, `PciExpress = 0x10`, `MsiX = 0x11`, `SataData = 0x12`, `AdvancedFeatures = 0x13`.
  - `PciCapability` record structure and linked-list extraction with infinite-loop cycle prevention.
- `kernel/src/pci/device.rs`: Comprehensive `PciDevice` Representation:
  - Physical configuration snapshot: vendor ID, device ID, class code, subclass, prog IF, revision, header type, subsystem vendor/ID, interrupt line & pin.
  - Probed BAR array (`[PciBar; 6]`) and parsed capability vector.
  - Driver helpers: `enable_bus_mastering()`, `enable_memory_space()`, `enable_io_space()`, `disable_interrupts()`, `format_description()`.
- `kernel/src/pci/manager.rs`: PCI Subsystem Manager (`PciManager`, `PCI_MANAGER`):
  - Recursive bus scan iterating across primary buses (0..8) and discovering downstream buses through PCI-to-PCI bridge headers.
  - Multi-function device detection via `HEADER_TYPE` bit 7.
  - Device lookup methods: `find_device(vendor_id, device_id)`, `find_by_class(class, subclass)`, `devices()`, `device_count()`.
  - Formatted serial console hardware inventory logging on system boot.
- `kernel/src/pci/mod.rs`: PCI subsystem module root, `pci::init()`, and `is_pci_initialized()` query API.
- `kernel/src/pci/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Configuration mechanism #1 I/O port operation, byte alignment, and Host Bridge ID verification.
  - Test 2: Standard Header type 0x00 parsing, vendor/device ID matching, and Command register read/write modification.
  - Test 3: Base Address Register probing, sizing algorithm, MMIO 32-bit and Port I/O address decoding.
  - Test 4: Capability linked-list traversal and MSI / PCIe capability extraction.
  - Test 5: Full PCI bus enumeration and device registry verification (verifying 7 physical PCI functions on VirtualBox VM).

### Changed
- `kernel/src/drivers/wrappers.rs`: Updated `PciFoundationDevice::status()` to return `HardwareSupportStatus::Supported` dynamically once `pci::is_pci_initialized()` is true.
- `kernel/src/main.rs`: Registered `pub mod pci;`, initialized PCI subsystem with `pci::init();`, and executed `pci::tests::run_pci_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 23 — Networking (2026-11-15)

### Added
- `kernel/src/net/types.rs`: Core network address definitions and link-layer types:
  - `MacAddress`: 48-bit IEEE 802.3 MAC hardware address with string parsing (`52:54:00:12:34:56`), formatting, broadcast (`FF:FF:FF:FF:FF:FF`), and unicast detection.
  - `Ipv4Address`: 32-bit Internet Protocol address with dotted decimal string parsing, formatting, subnet mask matching (`is_in_subnet`), broadcast, and loopback (`127.0.0.1`).
  - `EtherType`: Standard link-layer protocol identifiers (`Ipv4 = 0x0800`, `Arp = 0x0806`, `Ipv6 = 0x86DD`).
  - `IpProtocol`: Transport-layer identifiers (`Icmp = 1`, `Tcp = 6`, `Udp = 17`).
  - `SocketAddr`: IPv4 socket endpoint representation (`ip: Ipv4Address, port: u16`).
  - `NetError`: Subsystem error classifications (`BufferTooSmall`, `InvalidChecksum`, `PacketMalformed`, `ArpNotFound`, `PortInUse`, `ConnectionRefused`, `Timeout`, `DeviceError`, `SocketClosed`, `NotConnected`).
- `kernel/src/net/checksum.rs`: RFC 1071 Internet Checksum algorithms:
  - `internet_checksum`: 16-bit one's complement addition for IPv4 and ICMP headers with trailing odd-byte padding.
  - `transport_checksum`: 16-bit checksum over the IPv4 pseudo-header and UDP/TCP segment payload.
  - `verify_transport_checksum`: One's complement sum validation against pseudo-header and segment bytes.
- `kernel/src/net/ethernet.rs`: IEEE 802.3 Ethernet II Framing:
  - 14-byte frame header serialization and deserialization with destination MAC, source MAC, and EtherType.
  - `build_ethernet_frame`: Frame construction with minimum 60-byte payload padding.
- `kernel/src/net/arp.rs`: RFC 826 Address Resolution Protocol (ARP):
  - 28-byte ARP packet parser and builder supporting ARP Request (`Opcode 1`) and ARP Reply (`Opcode 2`).
  - `ArpTable`: Dynamic in-memory ARP cache table mapping IP to MAC addresses with automatic learning and special loopback/broadcast handling.
- `kernel/src/net/ipv4.rs`: RFC 791 Internet Protocol version 4:
  - 20-byte base IPv4 packet header parser and builder with dynamic IHL, total length, Don't Fragment flag, TTL, header checksum, and IP routing.
  - `build_ipv4_packet`: Complete IPv4 packet construction with automated checksum calculation.
- `kernel/src/net/icmp.rs`: RFC 792 Internet Control Message Protocol (ICMP):
  - ICMP Echo Request (`Type 8`) and Echo Reply (`Type 0`) packet generation and validation with sequence numbers and payload integrity.
  - Automated loopback echo reply generation for ping diagnostics.
- `kernel/src/net/udp.rs`: RFC 768 User Datagram Protocol (UDP):
  - 8-byte UDP header parsing and serialization with IPv4 pseudo-header checksum verification.
  - `UdpSocketTable`: Bound port multiplexing and datagram receive buffering.
- `kernel/src/net/tcp.rs`: RFC 793 Transmission Control Protocol (TCP):
  - 20-byte base TCP header parsing and segment generation with control flags (`SYN`, `ACK`, `FIN`, `RST`, `PSH`, `URG`).
  - `TcpState`: RFC 793 connection state machine (`Closed`, `Listen`, `SynSent`, `SynReceived`, `Established`, `CloseWait`, `FinWait1`, `FinWait2`, `TimeWait`).
  - Full 3-way handshake simulation (`Listen` -> `SynReceived` -> `Established`), data segment acknowledgment, and `FIN` connection teardown.
- `kernel/src/net/device.rs`: Network Device Abstraction & Software Loopback:
  - `NetDevice` trait defining hardware transmission, reception, and MAC query methods.
  - `LoopbackDevice` (`lo0`): In-memory virtual adapter with internal packet queue.
- `kernel/src/net/e1000.rs`: Intel 82540EM / 82545EM Gigabit Ethernet NIC Driver:
  - Hardware descriptor definitions (`RxDesc`, `TxDesc`) and register offset mappings (CTRL, STATUS, RCTL, TCTL, RDBAL, TDBAL, RAL, RAH).
  - Software packet queue integration with `NetDevice` trait.
- `kernel/src/net/interface.rs`: Interface configuration (`InterfaceConfig`):
  - Local IP address, Subnet Mask, Default Gateway, and DNS server definitions (`virtualbox_default`, `loopback`).
- `kernel/src/net/stack.rs`: Central Network Stack (`NetworkStack`, `NETWORK_STACK`):
  - Thread-safe ingress demultiplexing: Ethernet -> ARP / IPv4 -> ICMP / UDP / TCP.
  - Egress multiplexing: Socket / ping -> Transport -> IPv4 -> ARP resolution -> Ethernet frame transmission.
- `kernel/src/net/socket.rs`: High-Level Socket API:
  - Unified socket descriptor table with `socket`, `bind`, `send_to`, `recv_from`, and `close`.
- `kernel/src/net/mod.rs`: Subsystem module definitions, initialization (`net::init()`), and `is_network_initialized()` probe.
- `kernel/src/net/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: Link-layer MAC addressing, IPv4 addressing, Ethernet II framing & checksums.
  - Test 2: RFC 826 ARP request/reply generation and dynamic ARP cache table resolution.
  - Test 3: IPv4 packet construction, routing & automated ICMP Echo loopback (ping/pong).
  - Test 4: UDP datagram transmission, pseudo-header checksum, and socket port multiplexing.
  - Test 5: TCP 3-way handshake state transitions (Listen -> SynReceived -> Established) & teardown.

### Changed
- `kernel/src/drivers/wrappers.rs`: Updated `NetworkFoundationDevice::status()` to return `HardwareSupportStatus::Supported` dynamically once `net::is_network_initialized()` is true.
- `kernel/src/main.rs`: Registered `pub mod net;`, called `net::init();`, and executed `net::tests::run_net_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 22 — Desktop Applications (2026-11-15)

### Added
- `kernel/src/apps/terminal.rs`: Virtual Terminal application (`TerminalApp`):
  - Ring 0 interactive command shell with persistent history buffer up to 120 lines.
  - Built-in diagnostic and management commands: `help`, `uname`, `uptime` (ticks & seconds), `clear`, `echo`, `mem` (physical frames & kernel heap bytes), `ls` (directory listing), `cat` (text file output), `exit`.
  - Rich line editing: key navigation (Left, Right, Home, End), character insertion, backspace deletion, and dynamic prompt prefix (`integrate@os:<cwd>$ `).
  - High-contrast console rasterization with dark background, accent prompt coloring, and underline cursor.
- `kernel/src/apps/file_manager.rs`: Graphical VFS Explorer (`FileManagerApp`):
  - Interactive file browser inspecting VFS directory trees (`/`, `/dev`, `/boot`, `/ramfs`).
  - Top navigation toolbar with `[Up]`, `[Root]`, `[Refresh]` actions and active path display.
  - Formatted file entry listing with directory badges (`[DIR]`), file badges (`[FILE]`), size metrics, and row selection highlighting.
  - Double-click and Enter-key directory traversal and status reporting.
- `kernel/src/apps/sysmon.rs`: Real-time System Monitor application (`SystemMonitorApp`):
  - Telemetry collection from physical frame allocator, kernel heap allocator, PIT heartbeat timer, and process manager.
  - Multi-tab UI architecture: "Resources" dashboard and "System Info" hardware specifications.
  - Visual resource gauges with proportional fill bars: Physical RAM utilization, Kernel Heap utilization, and animated CPU PIT pulse.
  - Periodic 2 Hz telemetry refresh and manual refresh button.
- `kernel/src/apps/editor.rs`: Multi-line Text Editor application (`TextEditorApp`):
  - Multi-line document buffer editing with cursor navigation and scroll offset.
  - Full keyboard handling: line splitting on Enter, line merging on Backspace, character insertion, and arrow keys.
  - System clipboard integration: copying active lines (`Ctrl+C` / `[Copy]`) and pasting clipboard text (`Ctrl+V` / `[Paste]`).
  - VFS persistence: `load_file` reading text from VFS and `save_file` writing buffers to storage with dirty state tracking.
  - Editor visual layout: line number gutter with separator line, syntax coloring for comments/keywords, and bottom position status bar.
- `kernel/src/apps/settings.rs`: System Settings Control Panel (`SettingsApp`):
  - Desktop appearance theme selection (`Integrate Dark`, `Slate Blue`, `High Contrast`) via custom radio group controls.
  - Mouse pointer sensitivity calibration slider (1..10 scale) with interactive track and thumb dragging.
  - Desktop effect toggles using checkbox widgets: Window Drop Shadows, System Audio Effects, Window Animations.
  - Bottom action controls: `[Apply]` configuration changes and `[Defaults]` factory restoration.
- `kernel/src/apps/mod.rs`: Desktop Applications subsystem interface and `spawn_by_id` launcher factory.
- `kernel/src/apps/tests.rs`: Comprehensive 5-stage verification test suite:
  - Test 1: TerminalApp command execution, line editing, and canvas rasterization.
  - Test 2: FileManagerApp VFS browsing, path navigation, and entry selection.
  - Test 3: SystemMonitorApp live metrics sampling, progress gauges, and periodic updates.
  - Test 4: TextEditorApp multi-line editing, clipboard copy/paste, and VFS file persistence.
  - Test 5: SettingsApp control panel interaction and multi-app orchestration in AppManager.

### Changed
- `kernel/src/framework/app.rs`: Added custom canvas rendering hook `fn render(&self, _canvas: &mut Canvas)` to the `Application` trait with a default empty implementation.
- `kernel/src/framework/manager.rs`: Updated `AppManager::render_instance_ui` to invoke `instance.app.render(&mut canvas)` after widget rasterization.
- `kernel/src/graphics/canvas.rs`: Added `draw_rect_fill` and `draw_rect_outline` helper aliases.
- `kernel/src/main.rs`: Registered `pub mod apps;`, called `apps::init();`, and executed `apps::tests::run_apps_tests();`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 21 — Application Framework (2026-11-15)

### Added
- `kernel/src/framework/types.rs`: Core application lifecycle types:
  - `InstanceId`: Monotonically allocated strongly typed instance identifier.
  - `AppState`: Lifecycle execution states (`Uninitialized`, `Running`, `Suspended`, `Terminated`).
  - `AppError`: Lifecycle error representations (`InitializationFailed`, `WindowCreationFailed`, `InstanceNotFound`, `AlreadyTerminated`, `ResourceExhausted`).
- `kernel/src/framework/event.rs`: Unified application event envelope (`AppEvent`):
  - `WindowResized`, `WindowFocused`, `WindowUnfocused`.
  - `Input(WidgetEvent)`: Dispatched directly to application root UI container.
  - `Action(u32)`: UI interactive callbacks fired by button clicks, sliders, and entries.
  - `TimerTick(u64)`: Periodic timer notification.
  - `Custom { message_id, payload }`: Inter-process and custom IPC messaging.
  - `QuitRequested`: OS or user application termination request.
- `kernel/src/framework/clipboard.rs`: Thread-safe system-wide clipboard service:
  - Text copy/paste buffer with 4096-byte limit enforcement (`set_text`, `get_text`, `has_text`, `clear`).
  - Global static `CLIPBOARD` protected by `IrqSpinlock`.
- `kernel/src/framework/env.rs`: Application execution environment (`AppEnv`):
  - Bridges active application instance with its host window in `WindowManager`.
  - Hosts root UI `Container` with automated layout and widget binding.
  - Methods for widget addition (`add_widget`), window title synchronization (`set_title`), redraw requests (`request_redraw`), and clean exit requests (`request_quit`).
- `kernel/src/framework/app.rs`: Polymorphic application interface trait (`Application`):
  - `name(&self) -> &str`.
  - Lifecycle hooks: `on_init(&mut env)`, `on_update(&mut env, delta_ms)`, `on_event(&mut env, event)`, `on_destroy(&mut env)`.
- `kernel/src/framework/manager.rs`: Central application runtime coordinator (`AppManager`):
  - `spawn`: Instantiates window in `WindowManager`, sets up `AppEnv`, executes `on_init`, and renders initial UI container into the window client buffer.
  - `terminate`: Invokes `on_destroy`, transitions state to `Terminated`, and destroys associated host window.
  - `suspend` and `resume`: Safe task execution pausing and resumption.
  - `dispatch_event`: Routes input events through UI widget tree and delivers `Action` / `Custom` / `Timer` events to application instances.
  - `update_all`: Ticks running instances with frame delta time, re-renders damaged UI containers, and processes automatic exit requests.
  - `render_instance_ui`: Offscreen rasterization of the application's root UI container into the host window's client buffer.
- `kernel/src/framework/mod.rs`: Application Framework subsystem interface and global static `APP_MANAGER`.
- `kernel/src/framework/tests.rs`: Comprehensive 5-stage verification suite:
  - Test 1: Application lifecycle state machine (Spawn, Suspend, Resume, Terminate) & monotonic IDs.
  - Test 2: AppEnv UI root container integration, widget binding & action callback routing.
  - Test 3: Thread-safe system clipboard copy/paste service & bounds enforcement.
  - Test 4: AppEvent bus, custom inter-process messaging & timer tick processing.
  - Test 5: Full application execution cycle with WindowManager client buffer rasterization & auto-exit.

### Changed
- `kernel/src/main.rs`: Registered `pub mod framework;`, initialized `framework::init()`, and executed `framework::tests::run_framework_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 20 — Integrate OS Desktop (2026-11-15)

### Added
- `kernel/src/desktop/types.rs`: Core desktop shell data types and action definitions:
  - `AppId`: Strongly typed enumeration of built-in applications (`Terminal`, `FileManager`, `SystemMonitor`, `TextEditor`, `Settings`) with display titles.
  - `DesktopAction`: Interaction action dispatched to system (`None`, `LaunchApp(AppId)`, `Shutdown`, `Reboot`, `ToggleStartMenu`).
  - `IconState`: Visual interaction state (`Normal`, `Hover`, `Selected`).
- `kernel/src/desktop/wallpaper.rs`: Desktop backdrop composition & branding watermark:
  - Dual-stop vertical linear gradient (`0xFF111D28` -> `0xFF1B2C3D`).
  - Minimalist geometric isometric cube wireframe emblem rendered in center-right region with nested isometric facets (`WATERMARK_COLOR`, `WATERMARK_ACCENT`).
- `kernel/src/desktop/icons.rs`: Desktop shortcut icons & grid manager (`DesktopIconGrid`):
  - Standard left-column icon grid layout with automatic vertical stacking.
  - Dedicated vector glyph rendering for built-in apps: Terminal (`>_` prompt in green), File Manager (folder tab & body in blue), System Monitor (ECG pulse waveform in emerald), Settings (gear with hub & spokes in slate), Text Editor (lined document sheet).
  - High-contrast text labels rendered with 1px drop shadows for legibility against wallpaper.
  - Selection highlights (translucent blue background with 1px accent outline) and hover states.
  - Double-click application launching with 50-tick (500 ms) threshold detection.
- `kernel/src/desktop/start_menu.rs`: Anchored Start Menu popup component:
  - Fixed-size floating card anchored above the taskbar Start button (`x = 4`, `width = 224`, `height = 264`).
  - Multi-layer drop shadow for elevation above desktop wallpaper and window surfaces.
  - Header banner with "Integrate OS" title and "v0.1.0 (x86_64)" architecture subtitle.
  - Categorized application launcher list with hover highlights and accent bullets.
  - System power action row at the bottom with styled "Shutdown" and "Reboot" buttons.
  - Automatic focus dismissal when clicking outside the menu card bounds.
- `kernel/src/desktop/shell.rs`: Central Desktop Shell coordinator (`DesktopShell`):
  - Coordinates wallpaper, desktop icons, and Start Menu popup.
  - Multi-layer event routing: dispatches clicks between Start Menu, open windows, and desktop icons.
  - WindowManager application window instantiation (`launch_app`): creates and focuses window surfaces pre-populated with initial app views (Terminal prompt, system diagnostics, filesystem explorer, control panel, or scratchpad).
  - Background rendering (Layer 0) and overlay rendering (Layer 4).
- `kernel/src/desktop/mod.rs`: Desktop subsystem interface and global static `DESKTOP_SHELL`.
- `kernel/src/desktop/tests.rs`: Comprehensive 5-stage verification suite:
  - Test 1: Desktop icon grid layout, hover, selection & double-click app launching.
  - Test 2: Start Menu anchoring, open/close lifecycle, outside click dismissal & item hit-testing.
  - Test 3: Application window instantiation and WindowManager integration.
  - Test 4: Desktop Shell multi-layer event routing hierarchy.
  - Test 5: Full desktop scene composition and pixel readback on Canvas surface.

### Changed
- `kernel/src/main.rs`: Registered `pub mod desktop;`, initialized `desktop::init()`, and executed `desktop::tests::run_desktop_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 19 — GUI Toolkit (2026-11-15)

### Added
- `kernel/src/gui/types.rs`: Core GUI Toolkit primitives & event system:
  - `WidgetId`: Strongly typed monotonic identifier.
  - `WidgetState`: `Normal`, `Hover`, `Pressed`, `Disabled`.
  - `Orientation` (`Horizontal`, `Vertical`) and `Alignment` (`Start`, `Center`, `End`, `Stretch`).
  - `ButtonStyle`: `Primary`, `Secondary`, `Danger`, `Success`.
  - `WidgetEvent`: `MouseMove`, `MouseDown`, `MouseUp`, `KeyDown`, `KeyUp`, `FocusGained`, `FocusLost`.
  - `EventResponse`: `Ignored`, `Consumed`, `Action(u32)`.
- `kernel/src/gui/theme.rs`: Dark-mode styling palette and control metrics:
  - Surface backgrounds (`BG_SURFACE`, `BG_PANEL`, `BG_INPUT`), borders, text typography (`TEXT_PRIMARY`, `TEXT_SECONDARY`, `TEXT_DISABLED`).
  - Button state colors (`BTN_PRIMARY_*`, `BTN_SECONDARY_*`, `BTN_DANGER_*`, `BTN_SUCCESS_*`, `BTN_DISABLED_BG`).
  - Control metrics: `CHECKBOX_SIZE` (16px), `RADIO_RADIUS` (7px), `SLIDER_TRACK_HEIGHT` (4px), `SLIDER_THUMB_RADIUS` (6px), `PROGRESS_BAR_HEIGHT` (14px).
- `kernel/src/gui/widget.rs`: Core polymorphic `Widget` trait:
  - Identity, bounds geometry (`bounds`, `set_bounds`), visibility, enablement, input focus, event handling (`handle_event`), and canvas rasterization (`render`).
- `kernel/src/gui/widgets/label.rs`: Text label widget supporting alignment (`Start`, `Center`, `End`), custom foreground/background colors, and vertical centering.
- `kernel/src/gui/widgets/button.rs`: Interactive push buttons with hover/pressed states, keyboard triggers (Enter/Space), focus highlights, and action firing.
- `kernel/src/gui/widgets/checkbox.rs`: Toggleable checkbox with custom checkmark vector path, focus rectangle, label text, and spacebar toggle support.
- `kernel/src/gui/widgets/radio.rs`: Mutually exclusive radio buttons with group identifiers, circular indicator with inner dot, and keyboard selection.
- `kernel/src/gui/widgets/progress.rs`: Visual progress bar with background trough, accent fill, value clamping, and centered percentage text ("65%").
- `kernel/src/gui/widgets/slider.rs`: Draggable horizontal slider with track, filled progress portion, circular thumb handle, value stepping, and arrow key navigation.
- `kernel/src/gui/widgets/text_input.rs`: Single-line editable text input with cursor positioning, character typing, backspace, delete, navigation keys (Left/Right/Home/End), Enter submission, and max length bounds.
- `kernel/src/gui/widgets/mod.rs`: Re-exports of all core widgets.
- `kernel/src/gui/layout.rs`: `BoxLayout` engine supporting both horizontal and vertical widget stacking with configurable padding, item spacing, and alignment (`Start`, `Center`, `End`, `Stretch`).
- `kernel/src/gui/container.rs`: Hierarchical `Container` panel widget managing child collections (`Vec<Box<dyn Widget>>`), background/border styling, automated box layout distribution, focus tracking, and event dispatch.
- `kernel/src/gui/mod.rs`: Central GUI toolkit subsystem interface and `gui::init()`.
- `kernel/src/gui/tests.rs`: Comprehensive 5-stage verification suite:
  - Test 1: Widget hierarchy, bounds, visibility, enablement & BoxLayout geometry (Vertical & Horizontal).
  - Test 2: Button state transitions (Normal -> Hover -> Pressed -> Click Action) & CheckBox / RadioButton selection.
  - Test 3: ProgressBar & Slider value clamping, mouse dragging & percentage calculations.
  - Test 4: TextInput typing, cursor navigation, backspace/delete & length bounds.
  - Test 5: Full Container hierarchy rasterization & pixel color inspection on Canvas surface.

### Changed
- `kernel/src/main.rs`: Registered `pub mod gui;`, initialized `gui::init()`, and executed `gui::tests::run_gui_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 18 — Compositor (2026-11-15)

### Added
- `kernel/src/compositor/damage.rs`: Screen damage tracking and dirty region management:
  - `DamageRegion`: Tracks dirty rectangles across frames, automatically merging overlapping/adjacent bounds to reduce draw call overhead.
  - Collapses to a single union bounding box when dirty rect count exceeds 16 to bound processing cost.
  - Supports `mark_dirty(Rect)`, `mark_fullscreen()`, `is_dirty()`, `rects()`, and `clear()`.
- `kernel/src/compositor/effects.rs`: Software visual effects and compositing math:
  - Porter-Duff Source-Over alpha blending (`blend_argb`) for true 32-bit ARGB color compositing.
  - Pixel opacity scaling (`apply_opacity`).
  - Multi-layer soft drop shadows (`draw_drop_shadow`): 4 concentric shadow shells with Gaussian-like quadratic alpha falloff (60 -> 35 -> 18 -> 8) and offset (3px right, 5px down) for authentic window elevation.
- `kernel/src/compositor/taskbar.rs`: Desktop taskbar, window switcher & system tray:
  - Fixed 30px bottom bar with semi-translucent dark slate background (`0xEE1E222B`) and 1px top highlight border.
  - "Integrate" Start menu button (`TaskbarHitResult::StartButton`) with rounded appearance and accent hover state.
  - Dynamic window tab list (`TaskbarHitResult::TaskItem(WindowId)`) synchronizing with `WindowManager`: active windows highlighted with a 2px blue accent underline (`0xFF3B82F6`).
  - System tray (`TaskbarHitResult::TrayClock`) displaying live CPU/PIT uptime clock formatted as `HH:MM:SS`.
  - Hit testing and automatic window state toggling (focus/restore minimized window or minimize active window).
- `kernel/src/compositor/engine.rs`: Central compositing engine (`CompositorEngine`):
  - Strict bottom-to-top rendering pipeline:
    1. Desktop wallpaper background (gradient or custom fill).
    2. Window drop shadows (rendered per visible window).
    3. Window frames, title bars, controls, and client contents (Z-ordered).
    4. Taskbar, Start button, window tabs, and system tray clock.
    5. Virtual mouse cursor sprite with boundary clipping.
  - Incremental dirty rectangle VRAM flushing (`canvas.present_rect`), skipping presentation when no damage is marked.
  - Performance telemetry: monotonic frame counter, total dirty pixels rendered, and presentation time accounting.
- `kernel/src/compositor/mod.rs`: Central compositor subsystem interface and global static `COMPOSITOR`.
- `kernel/src/compositor/tests.rs`: Comprehensive 5-test verification suite:
  - Test 1: Damage tracking, non-overlapping rect accumulation, and dirty union merging.
  - Test 2: Software alpha blending (Porter-Duff source-over) and multi-layer window drop shadow calculation.
  - Test 3: Multi-layer scene composition hierarchy (ensuring taskbar and cursor correctly occlude window layers).
  - Test 4: Taskbar hit-testing, window list synchronization, and interactive minimize/restore toggling.
  - Test 5: Compositor performance metrics, frame counting, and dirty rectangle VRAM presentation.

### Changed
- `kernel/src/memory/heap.rs`: Expanded `HEAP_INITIAL_SIZE` from 8 MiB to 32 MiB (8,192 physical 4 KiB frames) and `HEAP_MAX_SIZE` to 64 MiB to comfortably accommodate multiple concurrent full-screen 1024x768x32bpp (3.14 MiB) back buffers, window surfaces, and compositor scene canvases.
- `kernel/src/wm/tests.rs`: Added explicit deallocation `drop(test_canvas)` to prevent test buffer leakage across verification phases.
- `kernel/src/main.rs`: Registered `pub mod compositor;`, initialized `compositor::init()`, and executed `compositor::tests::run_compositor_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 17 — Window Manager (2026-11-15)

### Added
- `kernel/src/wm/types.rs`: Core types and flags:
  - `WindowId`: Monotonic strongly typed window identifier.
  - `WindowState`: `Normal`, `Minimized`, `Maximized`, `Hidden`.
  - `WindowFlags`: Bitflags for `RESIZABLE`, `MOVABLE`, `CLOSABLE`, `MINIMIZABLE`, `MAXIMIZABLE`, `ALWAYS_ON_TOP`, `BORDERLESS`, `MODAL`, and `DEFAULT`.
  - `HitTestResult`: `Nowhere`, `ClientArea`, `TitleBar`, `CloseButton`, `MaximizeButton`, `MinimizeButton`, `ResizeBorder(ResizeEdge)`.
  - `ResizeEdge`: `Left`, `Right`, `Top`, `Bottom`, `TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`.
- `kernel/src/wm/theme.rs`: Decoration metrics and theme palette:
  - Metrics: Title bar height (26 px), border width (2 px), resize handle size (6 px), button size (14 px).
  - Colors: Active/inactive title bar gradients, active/inactive borders, close/maximize/minimize buttons, and desktop wallpaper gradient.
- `kernel/src/wm/window.rs`: Managed window representation (`Window`):
  - Bounds, title, flags, state, saved bounds for restore, and offscreen client surface buffer (`content_buffer`).
  - Outer geometry, client rectangle calculation (`client_rect`), client dimensions (`client_size`).
  - Decoration bounds: `title_bar_rect`, `close_button_rect`, `maximize_button_rect`, `minimize_button_rect`.
  - Client surface drawing: `fill_client`, `set_client_pixel`, `get_client_pixel`, `draw_client_rect`, `draw_client_string` (using 8x16 bitmap font).
  - Accurate regional hit-testing (`hit_test`) returning control button, title bar, resize edge, or client area.
- `kernel/src/wm/render.rs`: Window and desktop rendering routines:
  - `render_desktop`: Full-screen vertical gradient wallpaper composition.
  - `render_window`: 2px outer border, active gradient / inactive solid title bar, anti-aliased title text, control buttons, and client surface blitting with strict clipping.
- `kernel/src/wm/manager.rs`: Central window manager (`WindowManager`):
  - Monotonic `WindowId` allocation.
  - Z-ordered window stack maintaining bottom-to-top hierarchy and active window tracking.
  - Dynamic focus elevation (`focus_window`) bringing target window to the top of the z-order.
  - Drag-move (`InteractionState::Moving`) and resize (`InteractionState::Resizing`) with minimum constraint enforcement.
  - Lifecycle state transitions: `minimize_window`, `maximize_window`, `restore_window`, `destroy_window`.
  - Multi-layer hit-testing (`hit_test`) searching top-to-bottom through visible windows.
  - Full desktop scene compositing with virtual mouse cursor overlay.
- `kernel/src/wm/mod.rs`: Window manager subsystem interface and global static `WINDOW_MANAGER`.
- `kernel/src/wm/tests.rs`: Comprehensive Phase 17 verification suite:
  - Test 1: Window creation, geometry, and client buffer operations.
  - Test 2: Z-order stacking hierarchy and dynamic focus elevation.
  - Test 3: Multi-layer overlapping hit-testing and control button targets.
  - Test 4: Window translation (move), resizing, and minimum dimension constraints.
  - Test 5: Lifecycle transitions (Maximize, Restore, Minimize, Close) and full canvas composition.

### Changed
- `kernel/src/main.rs`: Initialized `wm::init()` and executed `wm::tests::run_wm_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 16 — Input Subsystem (2026-11-15)

### Added
- `kernel/src/input/events.rs`: Unified input event architecture:
  - `KeyCode` enum defining full hardware-independent key spectrum (Letters, Numbers, F1..F12, Control, Locks, Navigation, Keypad).
  - `KeyAction` enum (`Press`, `Release`, `Repeat`).
  - `Modifiers` bitmask structure (`SHIFT`, `CTRL`, `ALT`, `SUPER`, `CAPS_LOCK`, `NUM_LOCK`, `SCROLL_LOCK`).
  - `KeyEvent` rich keyboard event container.
  - `MouseButton` (`Left`, `Right`, `Middle`, `Other`) and `MouseButtons` bitmask.
  - `MouseEvent` rich mouse motion and button container.
  - `InputEventKind` and `InputEvent` envelope with monotonic timestamp.
- `kernel/src/input/queue.rs`: Interrupt-safe circular event ring buffer (`InputEventQueue`):
  - Fixed capacity of 256 events with overwrite-oldest policy to prevent input lag.
  - Non-blocking `push`, `pop`, `peek`, `len`, `is_empty`, and `overflow_count`.
- `kernel/src/input/keyboard.rs`: PS/2 Scancode Set 1 state decoder (`KeyboardState`):
  - Multi-byte extended scancode tracking (`0xE0` prefixes for Arrow keys, Home, End, PageUp, PageDown, Insert, Delete, Right Ctrl, Right Alt, Super keys).
  - Modifier tracking (Left/Right Shift, Left/Right Ctrl, Left/Right Alt, Left/Right Super, CapsLock toggle, NumLock toggle).
  - Translation to ASCII with Shift and CapsLock capitalization logic.
- `kernel/src/input/mouse.rs`: Hardware PS/2 Mouse Controller Driver (`Ps2Mouse`, IRQ 12 / Vector 44):
  - 8042 auxiliary device initialization via ports `0x60` and `0x64`.
  - PIC unmasking for IRQ 2 (Cascade) and IRQ 12 (Mouse).
  - Streaming 3-byte packet state machine with bit 3 sync validation.
  - 9-bit sign-extended motion vector calculation (`dx`, `dy`).
  - Inverted Y delta mapping for screen coordinate consistency.
  - Button state change detection (Left, Right, Middle).
- `kernel/src/input/cursor.rs`: Virtual mouse cursor tracker (`VirtualCursor`):
  - Screen-space coordinate maintenance clamped strictly to `[0, width - 1]` and `[0, height - 1]`.
  - Standard 12x19 arrow cursor sprite bitmap (`CURSOR_SPRITE`) with transparent, black outline, and white interior pixels.
  - Boundary-clipped sprite rendering routine.
- `kernel/src/input/vfs_node.rs`: Character device nodes mounted at `/dev/input`:
  - `/dev/input/mice`: Emulates standard POSIX/Linux 3-byte mouse packets `[flags, dx, dy]`.
  - `/dev/input/event0`: Binary event streaming device exposing serialized 16-byte input records `[timestamp, type, code, value]`.
- `kernel/src/input/mod.rs`: Central input subsystem manager:
  - Event pushing, polling, cursor querying, and device registration.
  - `Ps2MouseDevice` wrapper implementing `Driver` and `MouseDriver` registered with `DEVICE_MANAGER`.
- `kernel/src/input/tests.rs`: Comprehensive Phase 16 verification suite:
  - Test 1: Keyboard scancode decoding, extended prefixes, and modifier states.
  - Test 2: PS/2 mouse 3-byte packet decoding, sign extension, and button bitmasks.
  - Test 3: Screen boundary clamping (1024x768) and sprite clipping.
  - Test 4: Input event queue FIFO buffering and overflow resilience.
  - Test 5: `/dev/input/mice`, `/dev/input/event0` VFS reading and Device Manager registration.

### Changed
- `kernel/src/drivers/device.rs`: Added `MouseDriver` trait (`position`, `buttons`).
- `kernel/src/fs/vfs.rs`: Added `add_child_node` default method to `VfsNode` trait.
- `kernel/src/fs/ramfs.rs`: Implemented `add_child_node` for `RamDirectory`.
- `kernel/src/fs/mod.rs`: Added `mount_node` method and exported `pub fn mount_node`.
- `kernel/src/arch/x86_64/keyboard.rs`: Routed raw scancodes to `crate::input::process_keyboard_byte`.
- `kernel/src/arch/x86_64/interrupts.rs`: Added `IRQ_MOUSE` routing to `crate::input::mouse::handle_mouse_interrupt`.
- `kernel/src/main.rs`: Initialized `input::init()` and executed `input::tests::run_input_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 15 — Filesystem + VFS (2026-11-15)

### Added
- `kernel/src/fs/vfs.rs`: Abstract Virtual Filesystem (VFS) specification:
  - `NodeType` enum (`File`, `Directory`, `CharDevice`, `BlockDevice`, `Symlink`).
  - `OpenFlags` bitflags (`READ`, `WRITE`, `CREATE`, `APPEND`, `TRUNCATE`) with `BitOr` and `BitOrAssign`.
  - `SeekFrom` offset positioning (`Start(u64)`, `Current(i64)`, `End(i64)`).
  - `Metadata` and `DirEntry` structures for filesystem attributes and directory listings.
  - `VfsNode` trait (`name`, `node_type`, `metadata`, `read`, `write`, `lookup`, `create`, `mkdir`, `remove`, `readdir`, `truncate`, `clone_node`).
- `kernel/src/fs/path.rs`: Canonical path normalization and decomposition routines:
  - `normalize_path`: Resolves multiple leading/consecutive slashes, trailing slashes, current directory `.` markers, and parent directory `..` traversals.
  - `split_path`: Safely separates canonical path into `(parent_path, child_name)`.
- `kernel/src/fs/ramfs.rs`: Dynamic in-memory tree filesystem (`RamFs`):
  - `RamFile`: Dynamic memory-backed file sharing thread-safe interior data buffer across clones and open handles (`Arc<IrqSpinlock<RamFileInner>>`).
  - `RamDirectory`: Dynamic memory-backed directory sharing children collection across clones (`Arc<IrqSpinlock<RamDirectoryInner>>`).
  - Dynamic inode allocation via atomic counter.
  - Directory node lookup, creation, subdirectory creation, removal, and enumeration (`.`, `..`, and children).
- `kernel/src/fs/devfs.rs`: Special device filesystem (`DevFs`) mounted at `/dev`:
  - `/dev/null`: Character sink discarding all writes and returning EOF (0 bytes) on read.
  - `/dev/zero`: Character generator returning infinite zero-filled bytes.
  - `/dev/console`: Terminal console routing writes to VGA text buffer and serial UART COM1, and reading from PS/2 keyboard queue.
  - `/dev/hda`: Hardware block storage device node backed by Phase 14 ATA/IDE driver, enforcing 512-byte sector reads and writes.
- `kernel/src/fs/file_table.rs`: File descriptor table (`FileTable`, `FileHandle`):
  - Standard file descriptors `0` (stdin), `1` (stdout), `2` (stderr).
  - Dynamic allocation of file descriptors (`fd >= 3`).
  - Per-handle read/write offset tracking and seek support.
- `kernel/src/fs/mod.rs`: Central thread-safe VFS manager (`VFS`, `VfsManager`):
  - Root directory mount with standard hierarchy: `/dev`, `/bin`, `/etc`, `/home`, `/tmp`, `/proc`.
  - Pre-populated `/etc/os-release` and `/etc/hostname`.
  - High-level VFS API (`open`, `close`, `read`, `write`, `seek`, `stat`, `mkdir`, `remove`, `readdir`, `read_file`, `write_file`).
- `kernel/src/syscall/numbers.rs`: Added VFS system call numbers:
  - `SYS_OPEN = 11`, `SYS_CLOSE = 12`, `SYS_SEEK = 13`, `SYS_STAT = 14`, `SYS_MKDIR = 15`.
- `kernel/src/syscall/dispatcher.rs`:
  - Extended dispatcher to handle `sys_open`, `sys_close`, `sys_seek`, `sys_stat`, `sys_mkdir`.
  - Routed `sys_write` and `sys_read` for `fd >= 3` to `crate::fs`.
  - Added `UserStat` C-compatible structure for `sys_stat`.
- `kernel/src/fs/tests.rs`: Comprehensive Phase 15 verification suite:
  - Test 1: VFS mount hierarchy & canonical path normalization (`/etc/os-release`, `/etc/hostname`, `/etc/../etc/./os-release`).
  - Test 2: File creation, writing 256-byte payload, and 100% byte-for-byte readback verification.
  - Test 3: Nested directory creation (`/home/developer/projects`), nested file write, and directory enumeration (`readdir` validating `.`, `..`, and `readme.txt`).
  - Test 4: File seek operations (`SeekFrom::Start`, `Current`, `End`), sparse gap writing, and partial byte reading.
  - Test 5: Special device nodes (`/dev/null`, `/dev/zero`, `/dev/hda` block storage), file removal, and bad FD error rejection.

### Changed
- `kernel/src/main.rs`: Initialized `fs::init()` and executed `fs::tests::run_fs_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 14 — Storage & Milestone E (2026-11-15)

### Added
- `kernel/src/storage/ata.rs`: Complete IDE / ATA PIO controller hardware driver:
  - Port definitions for Primary (`0x1F0`/`0x3F6`) and Secondary (`0x170`/`0x376`) IDE channels.
  - Drive interrogation via `IDENTIFY DEVICE` (0xEC), detecting ATA and ATAPI signatures and parsing model, serial, and 28-bit/48-bit LBA geometry.
  - Polled 28-bit LBA single-sector reads (`read_sector_pio`, command `0x20`) and writes (`write_sector_pio`, command `0x30`) with cache flush (`0xE7`).
  - Timeout-guarded BSY, DRQ, and ERR status polling.
- `kernel/src/storage/ramdisk.rs`: 128 KiB in-memory RAM disk block device (`RamDisk`, 256 sectors of 512 bytes) providing isolated, deterministic block storage.
- `kernel/src/storage/mod.rs`: Central storage subsystem manager (`STORAGE_MANAGER`):
  - Hardware bus discovery probing all 4 IDE channel configurations (Primary Master/Slave, Secondary Master/Slave).
  - Configures active primary block storage device (detects real VirtualBox ATA hard disk `'VBOX HARDDISK'`, 32,768 sectors / 16 MiB).
  - Block read/write API (`read_block`, `write_block`, `total_blocks`).
- `kernel/src/storage/tests.rs`: Comprehensive Milestone E verification suite:
  - Test 1: ATA controller probing and drive identification (ATAPI CD/DVD on Primary Master, real ATA hard disk on Primary Slave).
  - Test 2: Single-sector 512-byte write operation with unique magic pattern (LBA 1).
  - Test 3: Single-sector 512-byte readback and 100% byte-for-byte data integrity verification.
  - Test 4: Multi-sector read/write operations (4 contiguous sectors = 2048 bytes).
  - Test 5: Out-of-bounds LBA error handling and central driver registry verification (`IDE / ATA Block Storage Controller` updated to `SUPPORTED`).
- `scripts/vbox.ps1`: Added automatic 16 MiB virtual test disk creation (`integrate_disk.vdi`) and attachment to VirtualBox IDE controller on Primary Slave (`port 0, device 1`).

### Changed
- `kernel/src/drivers/wrappers.rs`: Implemented `StorageDriver` on `StorageFoundationDevice`, updating its operational status to `HardwareSupportStatus::Supported` when storage is initialized.
- `kernel/src/main.rs`: Initialized `storage::init()` and executed `storage::tests::run_storage_tests()`, verified live on VirtualBox VM `IntegrateOS_D`.

---

## Phase 13 — Scheduler & Multitasking (2026-11-15)

### Added
- `kernel/src/scheduler/priority.rs`: Task priority classification (`Priority`: `Realtime`, `High`, `Normal`, `Low`, `Idle`) with default timeslice calibration (10..200 ms).
- `kernel/src/scheduler/queue.rs`: Multi-level priority run queue (`RunQueue`) implementing prioritized FIFO scheduling across 5 priority buckets.
- `kernel/src/scheduler/sleep.rs`: Monotonic timer sleep queue (`SleepQueue`, `SleepEntry`), managing tick-based task suspension and automatic wakeup upon timer tick advancement.
- `kernel/src/scheduler/mod.rs`: Central preemptive scheduler (`Scheduler`, `SCHEDULER`), managing run/sleep queues, cooperative yielding (`yield_now`), timed suspension (`sleep_current`), task termination (`exit_current`), lock-free context switch handover (`prepare_reschedule`), preemption inhibition guards (`preempt_disable`, `preempt_enable`), and diagnostic accounting metrics (`stats`).
- `kernel/src/scheduler/mod.rs`: Kernel thread termination stub (`kernel_thread_exit_stub`) automatically intercepting normal `ret` from kernel thread functions and transitioning them to `ProcessState::Terminated(0)`.
- `kernel/src/scheduler/tests.rs`: Comprehensive Phase 13 verification suite:
  - Test 1: Run queue multi-level priority order & FIFO fairness
  - Test 2: Multi-task cooperative multitasking (`yield_now`) across 3 worker tasks (`task_alpha`, `task_beta`, `task_gamma`) executing 6 interleaved sequential handovers
  - Test 3: Sleep queue and timed task suspension (`sleep_current(5)`), sleeping 5 ticks and waking automatically
  - Test 4: Hardware timer-driven preemption via 100 Hz PIT timer interrupt, successfully preempting a CPU-bound worker spinning through 1.63M loop iterations
  - Test 5: Automatic kernel thread termination via `kernel_thread_exit_stub` and state inspection.

### Changed
- `kernel/src/process/pcb.rs`: Updated `ProcessControlBlock::new_kernel` to reserve 72 bytes (9 quadwords) on the stack, placing `kernel_thread_exit_stub` directly above `entry_fn` matching System V AMD64 ABI 16-byte alignment requirements.
- `kernel/src/process/switch.rs`: Added atomic interrupt disabling (`cli`) inside `switch_context_asm` immediately after saving flags, preventing interrupt race conditions during stack pointer transitions.
- `kernel/src/arch/x86_64/interrupts.rs`: Updated `IRQ_TIMER` dispatcher to send PIC EOI before calling `scheduler::check_timer_tick()`, enabling seamless timer-driven preemption.
- `kernel/src/syscall/dispatcher.rs`: Connected `sys_yield` to `crate::scheduler::yield_now()` and `sys_sleep` to `crate::scheduler::sleep_current()`.
- `kernel/src/main.rs`: Initialized `scheduler::init()` and executed `scheduler::tests::run_scheduler_tests()`, verified live in VirtualBox VM `IntegrateOS_D`.

---

## Phase 12 — System Calls & Milestone D (2026-11-15)

### Added
- `kernel/src/syscall/numbers.rs`: System call specification defining canonical numbers (`SYS_EXIT = 1`, `SYS_WRITE = 2`, `SYS_READ = 3`, `SYS_GETPID = 4`, `SYS_GETPPID = 5`, `SYS_YIELD = 6`, `SYS_SLEEP = 7`, `SYS_UPTIME = 8`, `SYS_MMAP = 9`, `SYS_TEST_ECHO = 10`).
- `kernel/src/syscall/error.rs`: Strongly typed `SyscallError` enum mapping POSIX error codes (`Success = 0`, `EPERM = -1`, `ESRCH = -3`, `EBADF = -9`, `ENOMEM = -12`, `EFAULT = -14`, `EINVAL = -22`, `ENOSYS = -38`).
- `kernel/src/syscall/validate.rs`: User pointer security validator (`validate_user_pointer`, `validate_user_slice`, `validate_user_slice_mut`), enforcing address canonicality (`USER_SPACE_MIN = 0x10000`, `USER_SPACE_MAX = 0x0000_7FFF_FFFF_FFFF`), 64-bit arithmetic overflow checks, and 4-level MMU permission checks (`USER_ACCESSIBLE`, `PRESENT`, `WRITABLE`).
- `kernel/src/syscall/dispatcher.rs`: Central kernel system call dispatcher (`dispatch_syscall`) and handler implementations: `sys_exit`, `sys_write` (validating user slice, routing to VGA/serial), `sys_read` (reading PS/2 keyboard buffer), `sys_getpid`, `sys_getppid`, `sys_yield`, `sys_sleep` (calibrated against 100 Hz PIT), `sys_uptime`, and `sys_test_echo`.
- `kernel/src/syscall/fast_syscall.rs`: Hardware fast system call initialization (`init_fast_syscall`) programming `IA32_EFER.SCE` (System Call Extensions), `IA32_STAR` (Kernel CS `0x08`, Kernel SS `0x10`, User SS `0x1B`, User CS `0x23`), and `IA32_FMASK` (masking IF, DF, TF).
- `kernel/src/syscall/tests.rs`: Comprehensive Milestone D verification suite:
  - Test 1: System call dispatch & argument passing (`SYS_GETPID`, `SYS_GETPPID`, `SYS_UPTIME`, `SYS_TEST_ECHO`, and unknown syscall returning `-38` `ENOSYS`)
  - Test 2: `SYS_WRITE` to stdout (`fd = 1`) with validated user buffer, plus invalid file descriptor rejection (`-9` `EBADF`)
  - Test 3: Pointer security validation: NULL pointer (`0x0`), low IVT memory (`0x500`), supervisor text (`0x0010_0000`), supervisor heap (`0xFFFF_9000_...`), unmapped address (`0x0000_0000_7000_0000`), and overflowing pointer, all rejected with `-14` (`EFAULT`)
  - Test 4: Cooperative scheduling calls (`SYS_YIELD`, `SYS_SLEEP` measuring elapsed PIT ticks >= 5)
  - Test 5: Full Ring 3 User Mode execution via `int 0x80`, running user assembly testing echo, write, supervisor memory rejection, getpid, and clean exit.
- `kernel/src/syscall/mod.rs`: System call subsystem module declarations, re-exports, and initialization.

### Changed
- `kernel/src/memory/paging.rs`: Added `PageManager::get_page_flags` for dynamic virtual address permission verification across page table hierarchies.
- `kernel/src/user/mod.rs`: Extended `handle_user_syscall` (vector 128 / `int 0x80`) to decode standard 64-bit syscall registers (`RAX`/`RDI`, `RSI`, `RDX`, `R10`, `R8`), invoke `dispatch_syscall`, and return values via `RAX` to User Mode, returning cleanly to kernel mode on `SYS_EXIT`.
- `kernel/src/main.rs`: Initialized `syscall::init()` and executed `syscall::tests::run_syscall_tests()` during boot, verified live in VirtualBox VM `IntegrateOS_D`.

---

## Phase 11 — Processes (2026-11-15)

### Added
- `kernel/src/process/pid.rs`: Unique monotonic process identifier type (`ProcessId`, `next_pid`), defining standard constants for root kernel bootstrap (`ProcessId::KERNEL = 0`) and system manager (`ProcessId::INIT = 1`).
- `kernel/src/process/state.rs`: Process lifecycle state machine (`ProcessState`: `Ready`, `Running`, `Blocked`, `Sleeping`, `Terminated`), with `is_runnable()` and `is_terminated()` status predicates.
- `kernel/src/process/context.rs`: Task execution context (`TaskContext`) encapsulating callee-preserved registers (`rflags`, `r15..r12`, `rbp`, `rbx`, `rip`) for cooperative context switching.
- `kernel/src/process/switch.rs`: Low-level x86_64 context switch routine (`switch_context_asm`), saving outgoing registers and stack pointer, switching virtual address space (`CR3`) if changing, and restoring incoming registers and stack pointer.
- `kernel/src/process/pcb.rs`: Process Control Block (`ProcessControlBlock`) managing PID, PPID, name, state, priority, address space (`CR3`), dedicated 16 KiB kernel stack, user stack pointers, entry point, and CPU accounting statistics.
- `kernel/src/process/manager.rs`: Central thread-safe process table manager (`ProcessManager`, `PROCESS_MANAGER`), implementing kernel thread and user process creation (`spawn_kernel`, `spawn_user`), PID lookup, state transitions, termination, and process table enumeration.
- `kernel/src/process/tests.rs`: Comprehensive Phase 11 process verification suite:
  - Unique monotonic PID allocation (PIDs 0..3)
  - Process lifecycle state machine transitions (`Ready` -> `Running` -> `Blocked` -> `Ready` -> `Terminated`)
  - Address space and stack metadata isolation (Kernel vs User)
  - Active low-level context switch (`switch_context_asm`) with register and stack preservation verified via live handover to Task B and return to Task A
  - Process table enumeration and diagnostics logging.
- `kernel/src/process/mod.rs`: Process subsystem module declarations and initialization interface.

### Changed
- `kernel/src/main.rs`: Initialized `PROCESS_MANAGER`, registered the root kernel bootstrap process (PID 0), and executed the Phase 11 process verification suite in VirtualBox (VM `IntegrateOS_D`).

---

## Phase 10 — User Mode (2026-11-15)

### Added
- `kernel/src/user/trampoline.rs`: Assembly trampoline (`enter_user_mode_asm`) executing atomic privilege level transition from Kernel Mode (Ring 0) to User Mode (Ring 3) via `iretq` with user data selector `0x1B` (`ds, es, fs, gs, ss`), user code selector `0x23` (`cs`), and interrupts enabled (`rflags = 0x202`).
- `kernel/src/user/test.rs`: 64-bit User Mode test payload (`user_mode_code_start` .. `user_mode_code_end`) running in Ring 3, performing:
  - CPL verification (`CS & 3 == 3`)
  - 64-bit integer arithmetic computation (`(42 * 10) + 7 = 427`)
  - User stack push and pop integrity verification
  - Software interrupt system call roundtrip via `int 0x80` (`SYS_USER_LOG`, syscall 1)
  - Privileged instruction isolation test (`cli` raising `#GP`)
  - Supervisor memory protection test (reading `0x0010_0000` raising `#PF`)
  - Clean user-space exit via `int 0x80` (`SYS_USER_EXIT`, syscall 2)
  - Context save/restore continuation mechanism (`user_mode_setjmp_and_enter`, `user_mode_return_point`).
- `kernel/src/user/mod.rs`: User Mode subsystem manager (`setup_user_space`, `handle_user_syscall`, `run_user_mode_tests`), allocating user virtual addresses `USER_CODE_VIRT_ADDR` (`0x0000_0000_4000_0000`, 1 GiB) and `USER_STACK_VIRT_ADDR` (`0x0000_0000_8000_0000`, 2 GiB) with canonical lower-half user isolation.

### Changed
- `kernel/src/memory/paging.rs`: Enhanced `PageManager::map_page` to automatically propagate `PageTableFlags::USER_ACCESSIBLE` across all 4 paging levels (PML4, PDPT, PD, PT) when mapping user pages, ensuring MMU permission hierarchy permits Ring 3 execution.
- `kernel/src/arch/x86_64/exceptions.rs`: Updated `#GP` (vector 13) and `#PF` (vector 14) exception dispatchers to intercept and decode user-mode privilege violations (`state.cs & 3 == 3`), verifying hardware MMU enforcement on unauthorized instructions (`cli`) and supervisor memory reads without crashing the kernel.
- `kernel/src/arch/x86_64/idt.rs`: Routed software interrupt vector 128 (`0x80`) directly to `crate::user::handle_user_syscall`.
- `kernel/src/main.rs`: Invoked `user::run_user_mode_tests()` during boot and verified all 5 user mode tests pass live in VirtualBox (VM `IntegrateOS_D`).

---

## Phase 9 — Framebuffer + Graphics (2026-11-15)

### Added
- `kernel/src/arch/x86_64/io.rs`: x86_64 Port I/O instructions (`inb`, `outb`, `inw`, `outw`, `inl`, `outl`, `io_wait`) and PCI Type 1 Configuration Space access (`pci_read_u32`, `pci_write_u32`).
- `kernel/src/graphics/color.rs`: 32-bit ARGB/XRGB pixel format (`Color32`) supporting channel access, raw little-endian encoding, Porter-Duff alpha blending, linear color interpolation (`lerp`), and Integrate OS dark/accent color palettes.
- `kernel/src/graphics/rect.rs`: 2D geometric primitives (`Point`, `Size`, `Rect`) with intersection, union, bounds containment, insets, offsets, and boundary clipping.
- `kernel/src/graphics/font.rs`: Embedded 8x16 monospace bitmap font glyph table covering ASCII 0..127 (2048 bytes) with glyph lookup (`get_glyph`).
- `kernel/src/graphics/bga.rs`: Bochs / VirtualBox VBE (BGA) hardware controller driver via I/O ports `0x1CE`/`0x1CF` with mode switching and PCI BAR0 discovery for linear framebuffer physical base.
- `kernel/src/graphics/canvas.rs`: Double-buffered graphics engine (`Canvas`) managing heap-allocated back buffer, boundary clipping, 64-bit block VRAM presentation (`present`), dirty region damage tracking, and 2D drawing primitives:
  - Single pixel with bounds clipping and alpha compositing (`draw_pixel`, `draw_pixel_blend`)
  - Bresenham line algorithm (`draw_line`) and optimized scanline fills (`draw_hline`, `draw_vline`)
  - Outline and filled rectangles (`draw_rect`, `draw_filled_rect`)
  - Midpoint circle algorithm outline and filled spans (`draw_circle`, `draw_filled_circle`)
  - Horizontal and vertical linear color gradients (`draw_gradient_h`, `draw_gradient_v`)
  - 8x16 bitmap character and string rendering (`draw_char`, `draw_text`).
- `kernel/src/graphics/console.rs`: Graphical text console overlay (`FramebufferConsole`) supporting terminal character grid, cursor tracking, newlines, and scrolling.
- `kernel/src/graphics/demo.rs`: Automated Milestone G verification suite (5 tests) and high-definition 1024x768 desktop interface renderer.
- `kernel/src/graphics/mod.rs`: Top-level graphics module and thread-safe global graphics context (`GRAPHICS`).

### Changed
- `kernel/src/memory/heap.rs`: Expanded initial heap size (`HEAP_INITIAL_SIZE`) from 1 MiB to 8 MiB (and `HEAP_MAX_SIZE` to 32 MiB) to comfortably accommodate 3 MiB 1024x768x32 double-buffered surfaces.
- `kernel/src/drivers/device.rs`: Added `FramebufferDriver: DisplayDriver` trait specifying resolution, pitch, bpp, framebuffer address, and presentation.
- `kernel/src/drivers/wrappers.rs`: Implemented `BgaFramebufferDevice` concrete driver.
- `kernel/src/drivers/manager.rs`: Registered `BgaFramebufferDevice` in central device registry, bringing active devices to 10 (5 `SUPPORTED`, 5 `FOUNDATION`).
- `kernel/src/main.rs`: Integrated graphics initialization, executed Milestone G test suite, and verified live on VirtualBox VM `IntegrateOS_D` (1024x768 screenshot and serial verification).

---

## Phase 8 — Driver Architecture (2026-11-15)

### Added
- `kernel/src/drivers/traits.rs`: Mandatory hardware classification matrix (`HardwareSupportStatus`: `Supported`, `Partial`, `Foundation`, `Unsupported`).
- `kernel/src/drivers/device.rs`: Unified trait-based device abstractions (`Driver`, `DeviceCategory`, `TimerDriver`, `KeyboardDriver`, `DisplayDriver`, `StorageDriver`, `NetworkDriver`, `RtcDriver`, `AcpiDriver`).
- `kernel/src/drivers/wrappers.rs`: Concrete wrapper drivers for operational subsystems (`PitTimerDevice`, `Ps2KeyboardDevice`, `VgaDisplayDevice`, `Com1SerialDevice`) and foundation drivers (`StorageFoundationDevice`, `NetworkFoundationDevice`, `PciFoundationDevice`, `AcpiFoundationDevice`, `UsbFoundationDevice`).
- `kernel/src/drivers/manager.rs`: Central thread-safe device registry (`DeviceRegistry`, `DEVICE_MANAGER`), providing registration, category lookup, enumeration, and status aggregation.
- `kernel/src/drivers/mod.rs`: Driver framework module declarations.

### Changed
- `kernel/src/main.rs`: Initialized `DEVICE_MANAGER`, registered 9 hardware drivers (4 `SUPPORTED`, 5 `FOUNDATION`), and enumerated live devices and status counts to VGA screen and COM1 serial in VirtualBox.

---

## Phase 7 — Kernel Heap + Rust Alloc (2026-11-15)

### Added
- `kernel/src/memory/heap.rs`: Implemented linked-list kernel heap allocator (`LinkedListAllocator`) managing dedicated higher-half virtual address range `0xFFFF_9000_0000_0000 .. 0xFFFF_9000_0100_0000` (up to 16 MiB), with initial 1 MiB mapped during boot, 16-byte alignment, free block coalescing, thread-safety via `IrqSpinlock`, and Rust's `GlobalAlloc` trait implementation (`ALLOCATOR`).
- Enabled the standard `alloc` crate in `kernel/src/main.rs`.

### Changed
- `kernel/src/memory/mod.rs`: Exported `heap` module.
- `kernel/src/main.rs`: Connected kernel heap and executed Milestone C verification suite:
  - `Box` allocation, dereference, and drop.
  - Dynamic `Vec` capacity growth and index integrity check with 500 items.
  - Dynamic `String` creation and concatenation.
  - 100 consecutive allocate/free cycles verifying zero fragmentation/memory leaks (0 bytes allocated, 1,048,576 bytes free).
  - Verified live in VirtualBox (VM `IntegrateOS_D`).

---

## Phase 6 — Paging + Virtual Memory (2026-11-15)

### Added
- `kernel/src/memory/paging.rs`: 4-level x86_64 paging structures (`PageTable`, `PageTableEntry`, `PageTableFlags`), bitwise operator support, 9-bit address index decomposition (`PageIndices`), virtual-to-physical address translation (`translate`), dynamic virtual page mapping (`map_page`) allocating intermediate page tables (PDPT, PD, PT) via `FRAME_ALLOCATOR` and splitting 2 MiB huge pages into 4 KiB pages when needed, page unmapping (`unmap_page`), and individual TLB invalidation (`invlpg` / `flush_tlb`).

### Changed
- `kernel/src/memory/mod.rs`: Exported `paging` module alongside `frame_allocator` and `mmap`.
- `kernel/src/main.rs`: Initialized `PAGE_MANAGER` using CR3; verified virtual-to-physical translation on kernel text segment; tested dynamic mapping of non-identity higher-half address `0xFFFF_8000_0000_0000` to physical frame `0x139000`; performed verified write and readback of 64-bit signature `0x1234_5678_CAFE_BABE`; unmapped virtual address, flushed TLB, confirmed translation failure, and returned physical frame to allocator.

---

## Phase 5 — Physical Memory Management (2026-11-15)

### Added
- `kernel/src/memory/mod.rs`: Established modular kernel memory management subsystem.
- `kernel/src/memory/mmap.rs`: Multiboot E820 firmware memory map parser (`parse_mmap`), distinguishing usable RAM from reserved, ACPI, and firmware ROM ranges, and computing total usable memory and upper bounds.
- `kernel/src/memory/frame_allocator.rs`: Bitmap-backed 4 KiB physical page frame allocator (`PhysicalFrameAllocator`), tracking up to 1 GiB (262,144 frames) with a 32 KiB static bitmap, automatically reserving low 1 MiB and loaded kernel binary sections (`.text`, `.rodata`, `.data`, `.bss`, stack, page tables), protected by an `IrqSpinlock`.

### Changed
- `kernel/src/main.rs`: Integrated E820 memory map parsing and physical frame allocator initialization during boot; performed and verified allocation of 3 unique non-null physical frames (`0x136000`, `0x137000`, `0x138000`), verified free and deterministic re-allocation cycles, and verified live in VirtualBox.

---

## Phase 4 — Hardware Interrupts (2026-11-15)

### Added
- `kernel/src/arch/x86_64/pic.rs`: Dual 8259 Programmable Interrupt Controller (PIC) driver remapping Master IRQs 0..7 to vectors 32..39 and Slave IRQs 8..15 to vectors 40..47, with `send_eoi`, `unmask_irq`, `mask_irq`, and `disable_all`.
- `kernel/src/arch/x86_64/pit.rs`: 8254 Programmable Interval Timer (PIT) driver configured in Mode 3 (Square Wave) at 100 Hz on IRQ 0 (Vector 32), with tick tracking (`handle_timer_interrupt`, `get_ticks`).
- `kernel/src/arch/x86_64/keyboard.rs`: PS/2 Keyboard Controller driver on IRQ 1 (Vector 33) with Scancode Set 1 make/break code translation (US QWERTY), Shift/Caps-lock tracking, and circular character queuing (`KEYBOARD_QUEUE`).
- `kernel/src/arch/x86_64/interrupts.rs`: Hardware interrupt dispatcher routing IRQ 0 to PIT and IRQ 1 to keyboard, issuing PIC EOI acknowledgements, and tracking `TOTAL_INTERRUPTS`.
- `kernel/src/sync.rs`: Interrupt-safe spinlock (`IrqSpinlock`, `IrqSpinlockGuard`) and critical section primitive (`without_interrupts`) disabling local interrupts and safely restoring RFLAGS upon drop.

### Changed
- `kernel/src/arch/x86_64/idt.rs`: Updated `interrupt_dispatch` to route vectors 32..47 directly to `interrupts::dispatch_irq`.
- `kernel/src/main.rs`: Initialized hardware interrupts, enabled CPU interrupts (`sti`), and verified clean reception and acknowledgment of 10 live hardware timer ticks in VirtualBox (Milestone B).

---

## Phase 3 — IDT + CPU Exceptions (2026-11-14)

### Added
- `kernel/src/arch/x86_64/idt.rs`: 256-entry 64-bit Interrupt Descriptor Table (IDT) with 16-byte gate descriptors (`IdtEntry`), `lidt` loading, `common_interrupt_stub` assembly saving/restoring full 64-bit `CpuState`, ISR stubs for vectors 0..47, 128 (syscall), and 255 (spurious), wiring vector 8 (`#DF`) to dedicated TSS `IST1` stack.
- `kernel/src/arch/x86_64/exceptions.rs`: Architectural CPU exception dispatchers (`#DE`, `#DB`, `NMI`, `#BP`, `#OF`, `#BR`, `#UD`, `#NM`, `#DF`, `#TS`, `#NP`, `#SS`, `#GP`, `#PF`, etc.), `CR2` linear address and error code decoding on page faults, complete register diagnostic dumps on fatal exceptions, and non-destructive exception self-tests (`run_exception_self_tests`).
- `kernel/src/arch/x86_64/registers.rs`: Added `sidt` to inspect live IDTR limit and base register.
- `kernel/src/logger.rs`: Added `klog_excp!` colored diagnostic logging macro.

### Changed
- `kernel/src/arch/x86_64/mod.rs`: Initialized IDT (`idt::init()`) and added IDTR limit (`0x0FFF`) and base verification to `verify_cpu_architecture`.
- `kernel/src/main.rs`: Invoked `arch::x86_64::exceptions::run_exception_self_tests()` during boot and verified all 3 test exceptions (`#BP`, `#DE`, `#UD`) execute and recover cleanly in VirtualBox.

---

## Phase 2 — x86_64 CPU Architecture (2026-11-14)

### Added
- `kernel/src/arch/mod.rs` and `kernel/src/arch/x86_64/mod.rs`: Dedicated `x86_64` CPU architecture layer separated from generic kernel code, with runtime hardware verification (`verify_cpu_architecture`) and structured diagnostic logging (`log_architecture_state`).
- `kernel/src/arch/x86_64/privilege.rs`: `PrivilegeLevel` (`Ring0` .. `Ring3`) and `SegmentSelector` abstractions.
- `kernel/src/arch/x86_64/registers.rs`: Control Registers (`Cr0`, `Cr2`, `Cr3`, `Cr4`), Model Specific Registers (`Msr`: `IA32_EFER`, `IA32_STAR`, `IA32_LSTAR`, `IA32_FMASK`), `RFlags`, segment & descriptor table readers (`read_cs`, `read_ds`, `read_ss`, `read_tr`, `sgdt`), and `CPUID` processor feature & vendor identification (`CpuInfo`).
- `kernel/src/arch/x86_64/state.rs`: Full 64-bit `CpuState` (`r15..rax`, `vector`, `error_code`, `rip`, `cs`, `rflags`, `rsp`, `ss`) and `HardwareInterruptFrame`.
- `kernel/src/arch/x86_64/tss.rs`: 104-byte 64-bit `TaskStateSegment` with dedicated 16 KiB stacks for Ring 3 -> Ring 0 transitions (`RSP0`), Double Faults (`IST1`), and critical faults (`IST2`).
- `kernel/src/arch/x86_64/gdt.rs`: 7-slot 64-bit GDT with Null (`0x00`), Kernel Code (`0x08`, Ring 0), Kernel Data (`0x10`, Ring 0), User Data (`0x1B`, Ring 3), User Code (`0x23`, Ring 3), and 16-byte 64-bit System TSS descriptor (`0x28`), loaded via `lgdt`, `retfq`, segment reloads, and `ltr`.

### Changed
- `kernel/src/main.rs`: Initialized and verified the Phase 2 `x86_64` CPU architecture layer during boot and verified GDT/TSS/Control Registers in VirtualBox.

---

## Phase 1 — GRUB + Rust Kernel Boot (2026-11-14)

### Added
- `kernel/src/boot.rs`: 32-bit Protected Mode Multiboot entry (`_start`), 4-level identity page table bootstrap (`boot_pml4`, `boot_pdpt`, `boot_pd` mapping 4 GiB via 2 MiB huge pages), PAE (`CR4.PAE`), Long Mode (`IA32_EFER.LME`), Paging + Write Protect (`CR0.PG | CR0.WP`), 64-bit bootstrap GDT, far transition (`long_mode_start`), 64 KiB kernel stack (`KERNEL_BOOT_STACK`), and `MultibootInfo` reader.
- `kernel/src/serial.rs`: 16550 UART serial driver (`COM1` at `0x3F8`, 115200 8N1, FIFO enabled) implementing `core::fmt::Write`.
- `kernel/src/vga.rs`: 80x25 VGA color text mode console driver (`0xB8000`) with scrolling and hardware CRT cursor updates (`0x3D4`/`0x3D5`).
- `kernel/src/logger.rs`: Dual-sink console macros (`kprint!`, `kprintln!`, `klog_info!`, `klog_ok!`) writing simultaneously to VGA screen and COM1 serial.
- `kernel/src/memory_layout.rs`: Linker-defined section boundary resolver (`__kernel_start` .. `__kernel_end`, `__stack_bottom` .. `__stack_top`) and `RSP`/`CR3` state verification.
- `kernel/src/main.rs`: 64-bit `kernel_main` verifying Multiboot magic `0x2BADB002`, printing the mandatory Phase 1 boot banner (`INTEGRATE OS / GRUB BOOT SUCCESS / 64-BIT KERNEL STARTED / KERNEL INITIALIZED`), validating 64-bit stack bounds, and providing a dual VGA/serial panic handler.

### Changed
- `kernel/linker.ld`: Placed `KEEP(*(.bss.page_tables))` and `KEEP(*(.bss.stack))` (`__stack_bottom` .. `__stack_top`) ahead of wildcard `*(.bss .bss.*)` so stack symbols enclose the full 64 KiB stack (`0x10B000 .. 0x11B000`). Included `*(.got .got.*)` in `.data`.
- `scripts/vbox.ps1`: Isolated VirtualBox test VM name to `IntegrateOS_D` inside `D:\IntegrateOS\build\vbox` and captured both `build\serial.log` and `build\vbox_screen.png`.

---

## Phase 0 — Development Foundation (2026-11-14)

### Added
- Clean workspace directory structure in `D:\IntegrateOS` after verifying directory was empty.
- Pinned Rust toolchain configuration (`rust-toolchain.toml`) targeting `x86_64-unknown-none` with `rust-lld` and `llvm-tools-preview`.
- Bare-metal Cargo configuration (`.cargo/config.toml`) with static relocation, kernel code model, `--nmagic`, and custom linker script (`kernel/linker.ld`).
- Custom `x86_64` linker script (`kernel/linker.ld`) placing the kernel at physical `1 MiB` (`0x00100000`) with `.multiboot` positioned within the first 8 KiB (`0x0190`).
- Initial `#![no_std]` / `#![no_main]` kernel crate (`kernel/Cargo.toml`, `kernel/src/main.rs`) containing the Multiboot1 header (`0x1BADB002`) and foundation `_start` entry symbol.
- GRUB2 configuration (`boot/grub/grub.cfg`) loading `/boot/integrate-os.bin` via `multiboot`.
- Reproducible PowerShell build, ISO generation, verification, and VirtualBox automation scripts (`scripts/build.ps1`, `scripts/iso.ps1`, `scripts/test.ps1`, `scripts/vbox.ps1`).
- Architectural and project documentation (`README.md`, `docs/ARCHITECTURE.md`, `PROJECT_STATUS.md`, `CHANGELOG.md`).
