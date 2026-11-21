//! Security & Isolation Types, Capabilities, and Resource Limits
//!
//! Provides:
//! - Fine-grained security capabilities (`Capability`, `CapabilitySet`)
//! - Memory pointer violation categories (`PointerViolation`)
//! - Process resource accounting and quotas (`ResourceLimits`, `ResourceLimitType`)
//! - Strongly typed security errors (`SecurityError`)
//! - Process security credentials and isolation context (`SecurityContext`)

use alloc::string::String;
use crate::process::pid::ProcessId;

/// Fine-grained system privileges and capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Capability {
    /// Read access to files, VFS mounts, and raw storage devices.
    ReadStorage        = 1 << 0,
    /// Write and modification access to files, directories, and storage devices.
    WriteStorage       = 1 << 1,
    /// Network socket operations (open, bind, connect, send, receive).
    NetworkAccess      = 1 << 2,
    /// Direct hardware port I/O (`inb`, `outb`) and physical MMIO mapping.
    HardwareIo         = 1 << 3,
    /// Process lifecycle control (spawn child, terminate, renice, signal).
    ManageProcesses    = 1 << 4,
    /// System administration (reboot, poweroff, load drivers, set system time).
    SystemAdmin        = 1 << 5,
    /// Audio playback and synthesizer hardware access.
    AudioAccess        = 1 << 6,
    /// GPU hardware acceleration and mode setting access.
    GpuAccess          = 1 << 7,
    /// ACPI sleep states, power management, and throttling.
    PowerManagement    = 1 << 8,
    /// Inter-process communication (pipes, shared memory, message queues).
    IpcCommunication   = 1 << 9,
    /// Read and export security audit trail logs.
    AuditLog           = 1 << 10,
    /// Raw network socket creation (bypassing transport layers).
    RawSockets         = 1 << 11,
    /// Dynamic kernel module or driver insertion/removal.
    KernelModules      = 1 << 12,
    /// Dynamic adjustment of process resource limits and quotas.
    SetResourceLimits  = 1 << 13,
}

impl Capability {
    /// Returns the raw bitmask representation of this capability.
    #[inline(always)]
    pub const fn bit(&self) -> u32 {
        *self as u32
    }

    /// Returns human-readable name of this capability.
    pub const fn name(&self) -> &'static str {
        match self {
            Capability::ReadStorage => "ReadStorage",
            Capability::WriteStorage => "WriteStorage",
            Capability::NetworkAccess => "NetworkAccess",
            Capability::HardwareIo => "HardwareIo",
            Capability::ManageProcesses => "ManageProcesses",
            Capability::SystemAdmin => "SystemAdmin",
            Capability::AudioAccess => "AudioAccess",
            Capability::GpuAccess => "GpuAccess",
            Capability::PowerManagement => "PowerManagement",
            Capability::IpcCommunication => "IpcCommunication",
            Capability::AuditLog => "AuditLog",
            Capability::RawSockets => "RawSockets",
            Capability::KernelModules => "KernelModules",
            Capability::SetResourceLimits => "SetResourceLimits",
        }
    }
}

/// Bitset of active security capabilities for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CapabilitySet(pub u32);

impl CapabilitySet {
    /// Empty capability set (zero privileges).
    pub const NONE: Self = Self(0);

    /// Full capability set (all system privileges, e.g. PID 0 Kernel/Supervisor).
    pub const ALL: Self = Self(0x3FFF); // Bits 0..13

    /// Standard unprivileged user application capabilities.
    pub const STANDARD_USER: Self = Self(
        Capability::ReadStorage.bit()
            | Capability::WriteStorage.bit()
            | Capability::NetworkAccess.bit()
            | Capability::AudioAccess.bit()
            | Capability::GpuAccess.bit()
            | Capability::IpcCommunication.bit(),
    );

    /// Strict isolated sandbox capabilities (zero hardware, storage write, or process management).
    pub const SANDBOX_STRICT: Self = Self(
        Capability::IpcCommunication.bit(),
    );

    /// Constructs a capability set from a raw bitmask.
    #[inline(always)]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns the raw bitmask.
    #[inline(always)]
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Returns true if the set contains the given capability.
    #[inline(always)]
    pub fn has(&self, cap: Capability) -> bool {
        (self.0 & cap.bit()) != 0
    }

    /// Grants a capability to this set.
    #[inline(always)]
    pub fn grant(&mut self, cap: Capability) {
        self.0 |= cap.bit();
    }

    /// Revokes a capability from this set.
    #[inline(always)]
    pub fn revoke(&mut self, cap: Capability) {
        self.0 &= !cap.bit();
    }

    /// Returns true if this capability set is a subset of `other`.
    #[inline(always)]
    pub fn is_subset(&self, other: &Self) -> bool {
        (self.0 & !other.0) == 0
    }

    /// Returns true if this capability set contains all capabilities in `required`.
    #[inline(always)]
    pub fn contains_all(&self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    /// Computes the intersection of two capability sets.
    #[inline(always)]
    pub fn intersect(&self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Computes the union of two capability sets.
    #[inline(always)]
    pub fn union(&self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Pointer validation failure classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerViolation {
    /// Null pointer access (address == 0).
    NullPointer,
    /// Address falls below `USER_SPACE_MIN` (0x10000, low 64 KiB protected BIOS/IVT).
    UnderflowLowMemory,
    /// Address falls in non-canonical range or higher-half kernel space.
    KernelAddressViolation,
    /// Arithmetic overflow on address + length calculation.
    ArithmeticOverflow,
    /// Buffer length is zero when non-empty was required.
    ZeroLength,
    /// Address is not properly aligned for type `T`.
    Unaligned { required: usize, actual: usize },
    /// Virtual page is not present in active page tables.
    PageNotPresent { addr: u64 },
    /// Virtual page lacks `USER_ACCESSIBLE` flag (supervisor only).
    SupervisorPage { addr: u64 },
    /// Write attempted on a read-only page (missing `WRITABLE` flag).
    WriteOnReadOnly { addr: u64 },
}

/// Resource limit category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimitType {
    /// Maximum virtual/heap memory allocation in bytes.
    MaxMemoryBytes,
    /// Maximum concurrently open file descriptors.
    MaxOpenFiles,
    /// Maximum child processes or threads.
    MaxChildProcesses,
    /// Maximum accumulated CPU execution time in milliseconds.
    MaxCpuTimeMs,
}

impl ResourceLimitType {
    pub const fn name(&self) -> &'static str {
        match self {
            ResourceLimitType::MaxMemoryBytes => "MaxMemoryBytes",
            ResourceLimitType::MaxOpenFiles => "MaxOpenFiles",
            ResourceLimitType::MaxChildProcesses => "MaxChildProcesses",
            ResourceLimitType::MaxCpuTimeMs => "MaxCpuTimeMs",
        }
    }
}

/// Per-process resource quotas and live usage accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_memory_bytes: u64,
    pub max_open_files: u32,
    pub max_child_processes: u32,
    pub max_cpu_time_ms: u64,

    pub allocated_memory_bytes: u64,
    pub open_files_count: u32,
    pub child_processes_count: u32,
    pub consumed_cpu_time_ms: u64,
}

impl ResourceLimits {
    /// Unlimited resource limits for kernel bootstrap / root supervisor.
    pub const UNLIMITED: Self = Self {
        max_memory_bytes: u64::MAX,
        max_open_files: u32::MAX,
        max_child_processes: u32::MAX,
        max_cpu_time_ms: u64::MAX,
        allocated_memory_bytes: 0,
        open_files_count: 0,
        child_processes_count: 0,
        consumed_cpu_time_ms: 0,
    };

    /// Standard resource limits for normal desktop user applications.
    pub const STANDARD_USER: Self = Self {
        max_memory_bytes: 256 * 1024 * 1024, // 256 MiB
        max_open_files: 128,
        max_child_processes: 32,
        max_cpu_time_ms: 3600 * 1000,        // 1 hour
        allocated_memory_bytes: 0,
        open_files_count: 0,
        child_processes_count: 0,
        consumed_cpu_time_ms: 0,
    };

    /// Strict resource limits for untrusted or isolated sandboxed processes.
    pub const STRICT_SANDBOX: Self = Self {
        max_memory_bytes: 16 * 1024 * 1024,  // 16 MiB
        max_open_files: 8,
        max_child_processes: 0,               // Cannot spawn children
        max_cpu_time_ms: 60 * 1000,          // 1 minute
        allocated_memory_bytes: 0,
        open_files_count: 0,
        child_processes_count: 0,
        consumed_cpu_time_ms: 0,
    };

    /// Constructs custom resource limits.
    pub fn new(max_mem: u64, max_files: u32, max_procs: u32, max_cpu: u64) -> Self {
        Self {
            max_memory_bytes: max_mem,
            max_open_files: max_files,
            max_child_processes: max_procs,
            max_cpu_time_ms: max_cpu,
            allocated_memory_bytes: 0,
            open_files_count: 0,
            child_processes_count: 0,
            consumed_cpu_time_ms: 0,
        }
    }

    /// Checks whether allocating `bytes` would exceed the memory limit.
    pub fn check_memory(&self, bytes: u64) -> Result<(), SecurityError> {
        let new_total = self.allocated_memory_bytes.saturating_add(bytes);
        if new_total > self.max_memory_bytes {
            Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxMemoryBytes))
        } else {
            Ok(())
        }
    }

    /// Charges memory allocation to the accounting counter.
    pub fn alloc_memory(&mut self, bytes: u64) -> Result<(), SecurityError> {
        self.check_memory(bytes)?;
        self.allocated_memory_bytes += bytes;
        Ok(())
    }

    /// Releases memory allocation from the accounting counter.
    pub fn free_memory(&mut self, bytes: u64) {
        self.allocated_memory_bytes = self.allocated_memory_bytes.saturating_sub(bytes);
    }

    /// Checks whether opening another file descriptor would exceed quota.
    pub fn check_file(&self) -> Result<(), SecurityError> {
        if self.open_files_count >= self.max_open_files {
            Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxOpenFiles))
        } else {
            Ok(())
        }
    }

    /// Charges an open file descriptor.
    pub fn alloc_file(&mut self) -> Result<(), SecurityError> {
        self.check_file()?;
        self.open_files_count += 1;
        Ok(())
    }

    /// Releases an open file descriptor.
    pub fn free_file(&mut self) {
        self.open_files_count = self.open_files_count.saturating_sub(1);
    }

    /// Checks whether spawning another child process would exceed quota.
    pub fn check_process(&self) -> Result<(), SecurityError> {
        if self.child_processes_count >= self.max_child_processes {
            Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxChildProcesses))
        } else {
            Ok(())
        }
    }

    /// Charges a child process.
    pub fn alloc_process(&mut self) -> Result<(), SecurityError> {
        self.check_process()?;
        self.child_processes_count += 1;
        Ok(())
    }

    /// Releases a terminated child process.
    pub fn free_process(&mut self) {
        self.child_processes_count = self.child_processes_count.saturating_sub(1);
    }

    /// Charges CPU time consumption.
    pub fn consume_cpu(&mut self, ms: u64) -> Result<(), SecurityError> {
        let new_total = self.consumed_cpu_time_ms.saturating_add(ms);
        if new_total > self.max_cpu_time_ms {
            Err(SecurityError::ResourceLimitExceeded(ResourceLimitType::MaxCpuTimeMs))
        } else {
            self.consumed_cpu_time_ms = new_total;
            Ok(())
        }
    }
}

/// Strongly typed security error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    /// Operation denied by security policy.
    PermissionDenied,
    /// Required capability is missing from the active capability set.
    MissingCapability(Capability),
    /// Process exceeded its configured resource quota.
    ResourceLimitExceeded(ResourceLimitType),
    /// User memory pointer or buffer validation failed.
    InvalidPointer(PointerViolation),
    /// Sandbox isolation constraint violated.
    SandboxViolation(&'static str),
    /// Target process context not found.
    ProcessNotFound(ProcessId),
    /// Privilege elevation attempted above bounding set.
    PrivilegeElevationForbidden,
}

/// Process security credentials, isolation state, and capability boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContext {
    pub pid: ProcessId,
    pub uid: u32,
    pub gid: u32,
    pub capabilities: CapabilitySet,
    pub bounding_set: CapabilitySet,
    pub limits: ResourceLimits,
    pub is_sandboxed: bool,
    pub name: String,
}

impl SecurityContext {
    /// Creates a privileged root supervisor context (PID 0).
    pub fn new_root(pid: ProcessId, name: &str) -> Self {
        Self {
            pid,
            uid: 0,
            gid: 0,
            capabilities: CapabilitySet::ALL,
            bounding_set: CapabilitySet::ALL,
            limits: ResourceLimits::UNLIMITED,
            is_sandboxed: false,
            name: String::from(name),
        }
    }

    /// Creates a standard user application context.
    pub fn new_user(pid: ProcessId, uid: u32, gid: u32, name: &str) -> Self {
        Self {
            pid,
            uid,
            gid,
            capabilities: CapabilitySet::STANDARD_USER,
            bounding_set: CapabilitySet::STANDARD_USER,
            limits: ResourceLimits::STANDARD_USER,
            is_sandboxed: false,
            name: String::from(name),
        }
    }

    /// Creates an isolated sandboxed application context.
    pub fn new_sandboxed(pid: ProcessId, name: &str) -> Self {
        Self {
            pid,
            uid: 65534, // Nobody / Sandbox UID
            gid: 65534,
            capabilities: CapabilitySet::SANDBOX_STRICT,
            bounding_set: CapabilitySet::SANDBOX_STRICT,
            limits: ResourceLimits::STRICT_SANDBOX,
            is_sandboxed: true,
            name: String::from(name),
        }
    }

    /// Returns true if this process currently holds `cap`.
    #[inline(always)]
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.has(cap)
    }

    /// One-way capability dropping: revokes `cap` from both effective capabilities and bounding set.
    /// Once dropped, the process can NEVER re-acquire this capability!
    pub fn drop_capability(&mut self, cap: Capability) -> Result<(), SecurityError> {
        self.capabilities.revoke(cap);
        self.bounding_set.revoke(cap);
        Ok(())
    }

    /// Updates effective capabilities, strictly bounded by `bounding_set`.
    pub fn set_capabilities(&mut self, caps: CapabilitySet) -> Result<(), SecurityError> {
        if !caps.is_subset(&self.bounding_set) {
            return Err(SecurityError::PrivilegeElevationForbidden);
        }
        self.capabilities = caps;
        Ok(())
    }

    /// Permanently drops privileges to at most `target_set`.
    pub fn drop_privileges_permanently(&mut self, target_set: CapabilitySet) -> Result<(), SecurityError> {
        self.bounding_set = self.bounding_set.intersect(target_set);
        self.capabilities = self.capabilities.intersect(self.bounding_set);
        Ok(())
    }

    /// Transitions this process context into an irrevocable strict sandbox.
    pub fn enter_strict_sandbox(&mut self) {
        self.is_sandboxed = true;
        self.drop_privileges_permanently(CapabilitySet::SANDBOX_STRICT).ok();
        if self.limits.max_memory_bytes > ResourceLimits::STRICT_SANDBOX.max_memory_bytes {
            self.limits.max_memory_bytes = ResourceLimits::STRICT_SANDBOX.max_memory_bytes;
        }
        if self.limits.max_open_files > ResourceLimits::STRICT_SANDBOX.max_open_files {
            self.limits.max_open_files = ResourceLimits::STRICT_SANDBOX.max_open_files;
        }
        self.limits.max_child_processes = 0;
    }
}
