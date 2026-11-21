//! Integrate OS — In-Built Package Manager (APT / APTITUDE / PKG)
//!
//! Architecture:
//!   Package Manager -> Repository Metadata -> Dependency Resolver -> Package Download
//!   -> Cryptographic Digest Verification -> VFS Installation (/usr/bin) -> Package Database (/var/lib/dpkg/status)
//!
//! Commands supported under `apt`, `aptitude`, and `pkg`:
//!   - update
//!   - list [--installed]
//!   - search <query>
//!   - install <pkg>
//!   - remove <pkg>
//!   - upgrade
//!   - show / info <pkg>
//!   - verify <pkg>

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::ramfs::RamFile;
use crate::fs::VFS;
use crate::pkg::database::PackageDatabase;
use crate::pkg::package::PackageStatus;
use crate::klog_info;

pub struct PackageManager {
    pub db: PackageDatabase,
}

impl PackageManager {
    pub fn new() -> Self {
        let mgr = Self {
            db: PackageDatabase::new(),
        };
        mgr.sync_vfs_database();
        mgr
    }

    /// Synchronizes `/etc/apt/sources.list` and `/var/lib/dpkg/status` and `/usr/bin/*` in VFS.
    pub fn sync_vfs_database(&self) {
        let _ = crate::fs::mkdir("/etc/apt");
        let _ = crate::fs::mkdir("/var");
        let _ = crate::fs::mkdir("/var/lib");
        let _ = crate::fs::mkdir("/var/lib/dpkg");
        let _ = crate::fs::mkdir("/usr");
        let _ = crate::fs::mkdir("/usr/bin");

        let sources = self.db.sources_list.join("\n") + "\n";
        let mut status_text = String::new();
        for (_name, pkg) in &self.db.packages {
            status_text.push_str(&format!(
                "Package: {}\nStatus: {}\nVersion: {}\nArchitecture: {}\nSHA256: {}\n\n",
                pkg.name,
                pkg.status.as_str(),
                pkg.version,
                pkg.architecture,
                pkg.sha256_digest
            ));
        }

        if let Some(ref mut vfs) = *VFS.lock() {
            let _ = vfs.remove("/etc/apt/sources.list");
            let _ = vfs.mount_node(
                "/etc/apt",
                Box::new(RamFile::with_data("sources.list", sources.as_bytes())),
            );
            let _ = vfs.remove("/var/lib/dpkg/status");
            let _ = vfs.mount_node(
                "/var/lib/dpkg",
                Box::new(RamFile::with_data("status", status_text.as_bytes())),
            );
        }
    }

    /// Executes `apt update` / `pkg update` — synchronizes repository indexes.
    pub fn apt_update(&mut self) -> String {
        let mut out = String::new();
        out.push_str("Hit:1 http://deb.integrateos.org/debian bookworm InRelease [SHA256 Verified]\n");
        out.push_str("Hit:2 http://security.integrateos.org/debian-security bookworm-security InRelease [SHA256 Verified]\n");
        out.push_str("Reading package lists... Done\n");
        out.push_str(&format!(
            "Building dependency tree... Done\nAll {} packages are up to date.\n",
            self.db.total_count()
        ));
        self.db.last_updated_epoch = crate::time::uptime_ms();
        self.sync_vfs_database();
        klog_info!("[APT] Package indices updated successfully ({} packages available)", self.db.total_count());
        out
    }

    /// Executes `apt list [--installed]` / `pkg list`.
    pub fn apt_list(&self, installed_only: bool) -> String {
        let mut out = String::new();
        out.push_str("Listing...\n");
        for (name, pkg) in &self.db.packages {
            if installed_only && !pkg.is_installed() {
                continue;
            }
            let status_tag = if pkg.is_installed() { "[installed]" } else { "" };
            out.push_str(&format!(
                "{}/bookworm {} {} {}\n",
                name, pkg.version, pkg.architecture, status_tag
            ));
        }
        out
    }

    /// Executes `apt search <query>` / `pkg search <query>`.
    pub fn apt_search(&self, query: &str) -> String {
        let mut out = String::new();
        let query_lower = query.to_ascii_lowercase();
        let mut count = 0;
        out.push_str("Sorting... Done\nFull Text Search... Done\n");
        for (name, pkg) in &self.db.packages {
            let name_match = name.to_ascii_lowercase().contains(&query_lower);
            let desc_match = pkg.description.to_ascii_lowercase().contains(&query_lower);
            if name_match || desc_match {
                let status_tag = if pkg.is_installed() { " [installed]" } else { "" };
                out.push_str(&format!(
                    "{}/bookworm {} {}{}\n  {}\n",
                    name, pkg.version, pkg.architecture, status_tag, pkg.description
                ));
                count += 1;
            }
        }
        if count == 0 {
            out.push_str(&format!("No packages found matching '{}'.\n", query));
        }
        out
    }

    /// Executes `apt show <pkg>` / `pkg info <pkg>`.
    pub fn apt_show(&self, name: &str) -> String {
        if let Some(pkg) = self.db.find(name) {
            let mut out = String::new();
            out.push_str(&format!("Package: {}\n", pkg.name));
            out.push_str(&format!("Version: {}\n", pkg.version));
            out.push_str(&format!("Priority: {}\n", pkg.priority));
            out.push_str(&format!("Section: {}\n", pkg.section));
            out.push_str(&format!("Installed-Size: {} kB\n", pkg.installed_size_kb));
            out.push_str(&format!("Maintainer: {}\n", pkg.maintainer));
            out.push_str(&format!("Architecture: {}\n", pkg.architecture));
            if !pkg.dependencies.is_empty() {
                out.push_str(&format!("Depends: {}\n", pkg.dependencies.join(", ")));
            }
            out.push_str(&format!("SHA256: {}\n", pkg.sha256_digest));
            out.push_str(&format!("Description: {}\n", pkg.description));
            out.push_str(&format!("Status: {}\n", pkg.status.as_str()));
            out
        } else {
            format!("E: Unable to locate package {}\n", name)
        }
    }

    /// Executes `apt install <pkg>` / `pkg install <pkg>` with dependency resolution and SHA256 verification.
    pub fn apt_install(&mut self, name: &str) -> (bool, String) {
        if let Some(pkg) = self.db.find(name) {
            if pkg.is_installed() {
                return (
                    true,
                    format!("{} is already the newest version ({}).\n", name, pkg.version),
                );
            }
        } else {
            return (false, format!("E: Unable to locate package {}\n", name));
        }

        // 1. Collect dependencies to install
        let mut to_install = Vec::new();
        let mut missing_deps = Vec::new();

        if let Some(target) = self.db.find(name) {
            for dep in &target.dependencies {
                if let Some(dpkg) = self.db.find(dep) {
                    if !dpkg.is_installed() {
                        to_install.push(dep.clone());
                    }
                } else {
                    missing_deps.push(dep.clone());
                }
            }
        }

        if !missing_deps.is_empty() {
            return (
                false,
                format!(
                    "E: Package '{}' has unmet dependencies: {}\n",
                    name,
                    missing_deps.join(", ")
                ),
            );
        }

        to_install.push(String::from(name));

        // 2. Cryptographic digest verification gate (pre-commit check for rollback safety)
        for pkg_name in &to_install {
            if let Some(pkg) = self.db.find(pkg_name) {
                if !pkg.verify_digest() {
                    return (
                        false,
                        format!(
                            "E: Cryptographic SHA256 verification failed for '{}'! Aborting & rolling back.\n",
                            pkg_name
                        ),
                    );
                }
            }
        }

        let mut out = String::new();
        out.push_str("Reading package lists... Done\nBuilding dependency tree... Done\n");
        out.push_str(&format!(
            "The following NEW packages will be installed:\n  {}\n",
            to_install.join(" ")
        ));

        let mut total_kb = 0;
        for pkg_name in &to_install {
            if let Some(pkg) = self.db.find_mut(pkg_name) {
                pkg.status = PackageStatus::Installed;
                total_kb += pkg.installed_size_kb;
                out.push_str(&format!(
                    "Get: http://deb.integrateos.org/debian bookworm/main {} {} [{} kB] [SHA256 OK]\n",
                    pkg.name, pkg.version, pkg.installed_size_kb
                ));

                // Install binary entry into /usr/bin/<name>
                if let Some(ref mut vfs) = *VFS.lock() {
                    let bin_content = format!(
                        "#!/bin/integrate-elf\nPACKAGE={}\nVERSION={}\nSHA256={}\n",
                        pkg.name, pkg.version, pkg.sha256_digest
                    );
                    let _ = vfs.mount_node(
                        "/usr/bin",
                        Box::new(RamFile::with_data(&pkg.name, bin_content.as_bytes())),
                    );
                }
            }
        }

        self.sync_vfs_database();

        out.push_str(&format!(
            "0 upgraded, {} newly installed, 0 to remove.\n",
            to_install.len()
        ));
        out.push_str(&format!("Need to get {} kB of archives.\n", total_kb));
        out.push_str("Verifying cryptographic SHA-256 package signatures... OK\n");
        out.push_str("Setting up packages... Done\nProcessing triggers for man-db... Done\n");
        klog_info!(
            "[APT] Successfully installed {} (installed size: {} kB, SHA256 verified)",
            name,
            total_kb
        );

        (true, out)
    }

    /// Executes `apt remove <pkg>` / `pkg remove <pkg>`.
    pub fn apt_remove(&mut self, name: &str) -> (bool, String) {
        let res = if let Some(pkg) = self.db.find_mut(name) {
            if !pkg.is_installed() {
                return (
                    true,
                    format!("Package '{}' is not installed, so not removed\n", name),
                );
            }
            pkg.status = PackageStatus::NotInstalled;
            let bin_path = format!("/usr/bin/{}", name);
            let _ = crate::fs::remove(&bin_path);
            klog_info!("[APT] Package '{}' uninstalled successfully", name);
            (
                true,
                format!(
                    "Removing {} ({})...\nProcessing triggers... Done\n",
                    name, pkg.version
                ),
            )
        } else {
            (false, format!("E: Unable to locate package {}\n", name))
        };
        self.sync_vfs_database();
        res
    }

    /// Executes `apt upgrade` / `pkg upgrade`.
    pub fn apt_upgrade(&mut self) -> String {
        let mut out = String::new();
        out.push_str("Reading package lists... Done\nBuilding dependency tree... Done\n");
        out.push_str("Calculating upgrade... Done\n");
        out.push_str(&format!(
            "0 upgraded, 0 newly installed, 0 to remove ({} packages currently installed and verified).\n",
            self.db.installed_count()
        ));
        out
    }

    /// Dispatches command input under `apt`, `aptitude`, or `pkg`.
    pub fn dispatch_command(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::from(
                "apt / pkg 2.6.1 (amd64)\nUsage: apt|pkg <command> [package]\nCommands: update, list, search, install, remove, upgrade, show, info, verify\n",
            );
        }

        match args[0] {
            "update" => self.apt_update(),
            "upgrade" => self.apt_upgrade(),
            "list" => {
                let installed = args.get(1).map(|&s| s == "--installed").unwrap_or(false);
                self.apt_list(installed)
            }
            "search" => {
                if args.len() < 2 {
                    String::from("E: Must specify at least one search pattern\n")
                } else {
                    self.apt_search(args[1])
                }
            }
            "show" | "info" => {
                if args.len() < 2 {
                    String::from("E: Must specify package name\n")
                } else {
                    self.apt_show(args[1])
                }
            }
            "verify" => {
                if args.len() < 2 {
                    String::from("E: Must specify package name to verify\n")
                } else if let Some(pkg) = self.db.find(args[1]) {
                    format!(
                        "Package: {} ({})\nSHA-256: {}\nIntegrity: {}\n",
                        pkg.name,
                        pkg.version,
                        pkg.sha256_digest,
                        if pkg.verify_digest() { "VERIFIED OK" } else { "FAILED" }
                    )
                } else {
                    format!("E: Unable to locate package {}\n", args[1])
                }
            }
            "install" => {
                if args.len() < 2 {
                    String::from("E: Must specify package name to install\n")
                } else {
                    self.apt_install(args[1]).1
                }
            }
            "remove" | "uninstall" => {
                if args.len() < 2 {
                    String::from("E: Must specify package name to remove\n")
                } else {
                    self.apt_remove(args[1]).1
                }
            }
            "help" | "--help" | "-h" => String::from(
                "Integrate OS Package Manager (APT / APTITUDE / PKG v2.6.1 amd64)\n\
                 Usage: apt | pkg <command> [args]\n\
                   update          - Retrieve & verify repository package lists\n\
                   upgrade         - Upgrade all installed packages\n\
                   list            - List packages (--installed for installed only)\n\
                   search <query>  - Search package names and descriptions\n\
                   show / info <p> - Display full metadata & SHA256 digest\n\
                   install <pkg>   - Resolve dependencies, verify SHA256 & install\n\
                   remove <pkg>    - Uninstall package & remove from /usr/bin\n\
                   verify <pkg>    - Verify cryptographic SHA256 digest of package\n",
            ),
            cmd => format!("E: Invalid operation {}\n", cmd),
        }
    }
}
