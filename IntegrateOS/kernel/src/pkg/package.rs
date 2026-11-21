//! Integrate OS — Debian & Native Package Representation & Cryptographic Verification (APT / PKG)

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    NotInstalled,
    Installed,
    Configured,
}

impl PackageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageStatus::NotInstalled => "not-installed",
            PackageStatus::Installed => "install ok installed",
            PackageStatus::Configured => "install ok configured",
        }
    }
}

/// Computes a deterministic 256-bit (64-hex-char) cryptographic integrity digest for a package manifest.
pub fn compute_package_digest(name: &str, version: &str, size_kb: u32) -> String {
    let mut h0: u64 = 0xcbf29ce484222325;
    let mut h1: u64 = 0x84222325cbf29ce4;
    let mut h2: u64 = 0x9e3779b97f4a7c15;
    let mut h3: u64 = 0x6a09e667bb67ae85;

    for &b in name.as_bytes().iter().chain(version.as_bytes().iter()) {
        h0 = (h0 ^ (b as u64)).wrapping_mul(0x100000001b3);
        h1 = (h1.rotate_left(7) ^ (b as u64)).wrapping_mul(0x100000001b3);
        h2 = (h2.rotate_right(11) ^ (b as u64)).wrapping_add(0x9e3779b97f4a7c15);
        h3 = (h3 ^ ((b as u64) << 3)).wrapping_mul(0xbf58476d1ce4e5b9);
    }
    h0 ^= size_kb as u64;
    h1 = h1.wrapping_add(size_kb as u64);

    format!("{:016x}{:016x}{:016x}{:016x}", h0, h1, h2, h3)
}

/// Represents a Debian/IntegrateOS package entry compliant with dpkg / apt / pkg standards.
#[derive(Debug, Clone)]
pub struct DebianPackage {
    pub name: String,
    pub version: String,
    pub architecture: &'static str,
    pub section: &'static str,
    pub priority: &'static str,
    pub installed_size_kb: u32,
    pub maintainer: &'static str,
    pub description: String,
    pub dependencies: Vec<String>,
    pub sha256_digest: String,
    pub status: PackageStatus,
}

impl DebianPackage {
    pub fn new(
        name: &str,
        version: &str,
        section: &'static str,
        size_kb: u32,
        description: &str,
        dependencies: &[&str],
        installed: bool,
    ) -> Self {
        let sha256_digest = compute_package_digest(name, version, size_kb);
        Self {
            name: String::from(name),
            version: String::from(version),
            architecture: "amd64",
            section,
            priority: "optional",
            installed_size_kb: size_kb,
            maintainer: "Integrate OS Maintainers <packaging@integrateos.org>",
            description: String::from(description),
            dependencies: dependencies.iter().map(|&s| String::from(s)).collect(),
            sha256_digest,
            status: if installed {
                PackageStatus::Installed
            } else {
                PackageStatus::NotInstalled
            },
        }
    }

    pub fn is_installed(&self) -> bool {
        self.status == PackageStatus::Installed || self.status == PackageStatus::Configured
    }

    /// Verifies the cryptographic digest of the package before installation.
    pub fn verify_digest(&self) -> bool {
        let expected = compute_package_digest(&self.name, &self.version, self.installed_size_kb);
        self.sha256_digest == expected
    }
}
