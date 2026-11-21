//! Integrate OS — Processor Affinity Architecture (Research Paper Incorporation)
//!
//! Research Topic: "Investigate how processor affinity and workload distribution affect
//! execution time and CPU utilization under different computational workloads on a Linux system"
//! Author: Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS (25PCCS201)
//!
//! Provides hardware-independent CPU affinity bitmasks, task-to-core binding rules,
//! and validation against logical CPU topology.

use alloc::string::String;
use alloc::vec::Vec;

/// Standard bitmask representing logical CPU core allocation.
/// Each bit position `i` corresponds to logical core `i` (0..63).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuAffinityMask(pub u64);

impl CpuAffinityMask {
    /// Default affinity across all 16 allowed logical CPUs (CPUs 0..15).
    /// Matches the Linux/WSL2 experimental environment reported in the research paper.
    pub const DEFAULT_ALL_ALLOWED: Self = Self(0x0000_FFFF);

    /// Restricted affinity: Single core CPU 0.
    pub const SINGLE_CPU_0: Self = Self(1 << 0); // 0x0001

    /// Restricted affinity: Four alternating cores (CPUs 0, 2, 4, 6).
    pub const FOUR_CPUS_0_2_4_6: Self = Self((1 << 0) | (1 << 2) | (1 << 4) | (1 << 6)); // 0x0055

    /// Creates an empty mask (no CPUs allowed).
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Creates an affinity mask from raw 64-bit integer.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns the raw 64-bit bitmask representation.
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Enables or disables scheduling on logical CPU `cpu_id`.
    pub fn set_cpu(&mut self, cpu_id: usize, allowed: bool) {
        if cpu_id < 64 {
            if allowed {
                self.0 |= 1u64 << cpu_id;
            } else {
                self.0 &= !(1u64 << cpu_id);
            }
        }
    }

    /// Returns `true` if logical CPU `cpu_id` is permitted by this affinity mask.
    pub fn is_cpu_allowed(&self, cpu_id: usize) -> bool {
        if cpu_id < 64 {
            (self.0 & (1u64 << cpu_id)) != 0
        } else {
            false
        }
    }

    /// Returns the total number of permitted logical CPU cores.
    pub fn count_allowed(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Computes intersection with system permitted CPUs (`set(cpus) & original_affinity`).
    pub fn intersect(&self, allowed: &Self) -> Self {
        Self(self.0 & allowed.0)
    }

    /// Formats the permitted CPU IDs as a comma-separated list matching the research paper:
    /// e.g. "0, 2, 4, 6".
    pub fn to_cpu_list_string(&self) -> String {
        let mut list = Vec::new();
        for cpu in 0..64 {
            if self.is_cpu_allowed(cpu) {
                let mut num = String::new();
                crate::research::workload::format_u64(&mut num, cpu as u64);
                list.push(num);
            }
        }
        if list.is_empty() {
            String::from("None")
        } else {
            list.join(", ")
        }
    }
}
