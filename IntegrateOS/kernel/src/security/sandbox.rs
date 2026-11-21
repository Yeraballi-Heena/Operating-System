//! Process Isolation & Sandbox Policy Enforcement
//!
//! Provides customizable containment policies for untrusted processes, preventing
//! unauthorized disk mutations, hardware I/O, network exfiltration, or process tree interference.

use crate::security::types::{Capability, CapabilitySet, SecurityContext};

/// Configurable sandbox containment profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub allow_storage_read: bool,
    pub allow_storage_write: bool,
    pub allow_network: bool,
    pub allow_hardware_io: bool,
    pub allow_process_manage: bool,
    pub allow_audio: bool,
    pub allow_gpu: bool,
    pub allow_ipc: bool,
    pub memory_limit_bytes: u64,
    pub max_open_files: u32,
    pub max_child_processes: u32,
}

impl SandboxPolicy {
    /// Strict sandbox: zero disk write, zero hardware I/O, zero network, zero process creation.
    pub const STRICT: Self = Self {
        allow_storage_read: false,
        allow_storage_write: false,
        allow_network: false,
        allow_hardware_io: false,
        allow_process_manage: false,
        allow_audio: false,
        allow_gpu: false,
        allow_ipc: true,
        memory_limit_bytes: 16 * 1024 * 1024, // 16 MiB
        max_open_files: 8,
        max_child_processes: 0,
    };

    /// Read-only filesystem policy: allows reading files and graphical display, forbids mutation.
    pub const READ_ONLY_FS: Self = Self {
        allow_storage_read: true,
        allow_storage_write: false,
        allow_network: false,
        allow_hardware_io: false,
        allow_process_manage: false,
        allow_audio: true,
        allow_gpu: true,
        allow_ipc: true,
        memory_limit_bytes: 64 * 1024 * 1024, // 64 MiB
        max_open_files: 32,
        max_child_processes: 0,
    };

    /// Pure compute isolation: no filesystem, no network, no peripherals.
    pub const COMPUTE_ONLY: Self = Self {
        allow_storage_read: false,
        allow_storage_write: false,
        allow_network: false,
        allow_hardware_io: false,
        allow_process_manage: false,
        allow_audio: false,
        allow_gpu: false,
        allow_ipc: true,
        memory_limit_bytes: 32 * 1024 * 1024,
        max_open_files: 4,
        max_child_processes: 0,
    };

    /// Translates this sandbox policy into an allowable capability set.
    pub fn to_capability_set(&self) -> CapabilitySet {
        let mut caps = CapabilitySet::NONE;
        if self.allow_storage_read {
            caps.grant(Capability::ReadStorage);
        }
        if self.allow_storage_write {
            caps.grant(Capability::WriteStorage);
        }
        if self.allow_network {
            caps.grant(Capability::NetworkAccess);
        }
        if self.allow_hardware_io {
            caps.grant(Capability::HardwareIo);
        }
        if self.allow_process_manage {
            caps.grant(Capability::ManageProcesses);
        }
        if self.allow_audio {
            caps.grant(Capability::AudioAccess);
        }
        if self.allow_gpu {
            caps.grant(Capability::GpuAccess);
        }
        if self.allow_ipc {
            caps.grant(Capability::IpcCommunication);
        }
        caps
    }

    /// Enforces the policy on a security context, permanently dropping prohibited capabilities
    /// and tightening resource bounds.
    pub fn apply(&self, ctx: &mut SecurityContext) {
        let allowed_caps = self.to_capability_set();
        ctx.drop_privileges_permanently(allowed_caps).ok();
        ctx.is_sandboxed = true;

        if ctx.limits.max_memory_bytes > self.memory_limit_bytes {
            ctx.limits.max_memory_bytes = self.memory_limit_bytes;
        }
        if ctx.limits.max_open_files > self.max_open_files {
            ctx.limits.max_open_files = self.max_open_files;
        }
        ctx.limits.max_child_processes = self.max_child_processes;
    }
}
