//! Integrate OS — ACPI Subsystem Types & Architecture Primitives
//!
//! Defines ACPI signatures, error types, Generic Address Structures (GAS),
//! power/sleep states, and power telemetry.

use core::fmt;

/// Standard 4-character ACPI Table Signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AcpiSignature(pub [u8; 4]);

impl AcpiSignature {
    pub const RSDT: Self = Self(*b"RSDT");
    pub const XSDT: Self = Self(*b"XSDT");
    pub const FADT: Self = Self(*b"FACP"); // FADT signature is "FACP"
    pub const DSDT: Self = Self(*b"DSDT");
    pub const MADT: Self = Self(*b"APIC"); // MADT signature is "APIC"
    pub const HPET: Self = Self(*b"HPET");
    pub const MCFG: Self = Self(*b"MCFG");
    pub const SSDT: Self = Self(*b"SSDT");
    pub const FACS: Self = Self(*b"FACS");

    pub const fn as_str(&self) -> &str {
        match core::str::from_utf8(&self.0) {
            Ok(s) => s,
            Err(_) => "????",
        }
    }
}

impl fmt::Debug for AcpiSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Display for AcpiSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// ACPI Generic Address Structure (GAS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct GenericAddress {
    /// 0 = System Memory, 1 = System I/O, 2 = PCI Config Space, 3 = Embedded Controller, 4 = SMBus
    pub address_space: u8,
    pub bit_width: u8,
    pub bit_offset: u8,
    pub access_size: u8,
    pub address: u64,
}

impl GenericAddress {
    pub const fn empty() -> Self {
        Self {
            address_space: 0,
            bit_width: 0,
            bit_offset: 0,
            access_size: 0,
            address: 0,
        }
    }

    pub fn is_memory(&self) -> bool {
        self.address_space == 0
    }

    pub fn is_io_port(&self) -> bool {
        self.address_space == 1
    }
}

/// Standard ACPI Sleep States.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepState {
    /// S0: Full working state
    S0,
    /// S1: Low power standby (CPU stops executing, caches maintained)
    S1,
    /// S2: Deeper standby (CPU powered down)
    S2,
    /// S3: Suspend to RAM (STR, context saved in RAM, system clock active)
    S3,
    /// S4: Suspend to Disk / Hibernation (STD, context written to storage)
    S4,
    /// S5: Soft Off / Full System Shutdown
    S5,
}

impl SleepState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::S0 => "S0 (Working)",
            Self::S1 => "S1 (Standby)",
            Self::S2 => "S2 (Deep Standby)",
            Self::S3 => "S3 (Suspend-to-RAM)",
            Self::S4 => "S4 (Hibernate)",
            Self::S5 => "S5 (Soft Off / Shutdown)",
        }
    }
}

/// System Power Source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    AcMains,
    Battery,
}

/// Battery Status Telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    pub present: bool,
    pub charging: bool,
    pub percentage: u8, // 0..100
}

impl BatteryStatus {
    pub const fn unknown() -> Self {
        Self {
            present: false,
            charging: false,
            percentage: 100,
        }
    }
}

/// ACPI Subsystem Error Taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    RsdpNotFound,
    ChecksumMismatch,
    TableNotFound,
    InvalidHeader,
    UnsupportedVersion,
    HardwareFault,
    InvalidSleepState,
}

impl AcpiError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RsdpNotFound => "Root System Description Pointer (RSDP) not found",
            Self::ChecksumMismatch => "ACPI table checksum verification failed",
            Self::TableNotFound => "Requested ACPI description table not found",
            Self::InvalidHeader => "Malformed ACPI standard description table header",
            Self::UnsupportedVersion => "Unsupported ACPI specification revision",
            Self::HardwareFault => "ACPI hardware register or controller access fault",
            Self::InvalidSleepState => "Unsupported or invalid ACPI sleep state transition",
        }
    }
}

impl fmt::Display for AcpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
