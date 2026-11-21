//! Integrate OS — Debian Package Database & Repository Index

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use crate::pkg::package::DebianPackage;

pub struct PackageDatabase {
    pub sources_list: Vec<String>,
    pub packages: BTreeMap<String, DebianPackage>,
    pub last_updated_epoch: u64,
}

impl PackageDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            sources_list: Vec::new(),
            packages: BTreeMap::new(),
            last_updated_epoch: 0,
        };

        db.sources_list.push(String::from("deb http://deb.integrateos.org/debian bookworm main contrib non-free"));
        db.sources_list.push(String::from("deb http://security.integrateos.org/debian-security bookworm-security main"));

        // Seed essential base packages
        db.register(DebianPackage::new(
            "coreutils", "9.1-1", "utils", 15420,
            "GNU core utilities for file, shell and text manipulation",
            &[], true
        ));
        db.register(DebianPackage::new(
            "bash", "5.2.15-2", "shells", 6450,
            "GNU Bourne Again SHell",
            &["coreutils"], true
        ));
        db.register(DebianPackage::new(
            "integrate-desktop", "1.0.0", "x11", 42300,
            "Integrate OS Linear Framebuffer Desktop Environment & Compositor",
            &["coreutils", "bash"], true
        ));
        db.register(DebianPackage::new(
            "wpa-supplicant", "2.10-12", "net", 3420,
            "Client support for WPA/WPA2/WPA3 (IEEE 802.11i) WiFi authentication",
            &["coreutils"], true
        ));
        db.register(DebianPackage::new(
            "net-tools", "2.10-0.1", "net", 1890,
            "Networking basic tools (ifconfig, route, netstat, arp)",
            &["coreutils"], true
        ));
        db.register(DebianPackage::new(
            "htop", "3.2.2-1", "utils", 320,
            "Interactive system-monitor and process-viewer with affinity display",
            &["coreutils"], false
        ));
        db.register(DebianPackage::new(
            "nano", "7.2-1", "editors", 780,
            "Small, friendly text editor inspired by Pico",
            &["coreutils"], false
        ));
        db.register(DebianPackage::new(
            "vim", "9.0.1378-2", "editors", 4200,
            "Vi IMproved - enhanced vi text editor",
            &["coreutils"], false
        ));
        db.register(DebianPackage::new(
            "curl", "7.88.1-10", "web", 650,
            "Command line tool for transferring data with URL syntax",
            &["net-tools"], false
        ));
        db.register(DebianPackage::new(
            "build-essential", "12.9", "devel", 48,
            "Informational list of build-essential packages (gcc, make, libc-dev)",
            &["coreutils"], false
        ));
        db.register(DebianPackage::new(
            "rustc", "1.75.0-1", "devel", 145000,
            "Rust systems programming language compiler",
            &["build-essential"], false
        ));

        db
    }

    pub fn register(&mut self, pkg: DebianPackage) {
        self.packages.insert(pkg.name.clone(), pkg);
    }

    pub fn find(&self, name: &str) -> Option<&DebianPackage> {
        self.packages.get(name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut DebianPackage> {
        self.packages.get_mut(name)
    }

    pub fn installed_count(&self) -> usize {
        self.packages.values().filter(|p| p.is_installed()).count()
    }

    pub fn total_count(&self) -> usize {
        self.packages.len()
    }
}
