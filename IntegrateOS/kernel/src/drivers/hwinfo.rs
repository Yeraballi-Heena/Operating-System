//! Integrate OS — Unified Hardware Discovery & PCI/PCIe Diagnostic Engine (Phases 1 & 2)
//!
//! Provides real hardware discovery (`hwinfo`) and live PCI/PCIe bus enumeration (`lspci`)
//! reading directly from CPUID, E820/FrameAllocator, PCI Configuration Space, ATA/NVMe/AHCI,
//! Network Controllers, USB Controllers, Audio, ACPI, and PIC/PIT timers.

use alloc::format;
use alloc::string::String;
use crate::arch::x86_64::pit;
use crate::arch::x86_64::registers::CpuInfo;
use crate::drivers::laptop::LAPTOP_MANAGER;
use crate::drivers::manager::DEVICE_MANAGER;
use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::heap::ALLOCATOR;
use crate::net::device::NetDevice;
use crate::net::stack::NETWORK_STACK;
use crate::net::wifi::WIFI_MANAGER;
use crate::pci::bar::Bar;
use crate::pci::PCI_MANAGER;

/// Generates real `lspci` output directly from enumerated PCI configuration space devices.
pub fn format_lspci(verbose: bool) -> String {
    let mut out = String::new();
    let pci = PCI_MANAGER.lock();

    if pci.devices.is_empty() {
        out.push_str("No PCI devices enumerated (ensure pci::init() has executed).\n");
        return out;
    }

    for dev in &pci.devices {
        let vendor_name = match dev.vendor_id {
            0x8086 => "Intel Corporation",
            0x80EE => "InnoTek Systemberatung GmbH (VirtualBox)",
            0x1022 => "Advanced Micro Devices, Inc. [AMD]",
            0x10DE => "NVIDIA Corporation",
            0x1234 => "QEMU / Bochs Technical",
            0x106B => "Apple Inc. (KeyLargo/Intrepid USB)",
            0x15AD => "VMware",
            _ => "PCI Vendor",
        };

        let dev_desc = match (dev.vendor_id, dev.device_id) {
            (0x8086, 0x1237) => "440FX - 82441FX PMC [Natoma] Host Bridge",
            (0x8086, 0x7000) => "82371SB PIIX3 ISA [Natoma/Triton II]",
            (0x8086, 0x7111) => "82371AB/EB/MB PIIX4 IDE Storage Controller",
            (0x8086, 0x7113) => "82371AB/EB/MB PIIX4 ACPI Power Controller",
            (0x8086, 0x100E) => "82540EM Gigabit Ethernet Controller (E1000)",
            (0x8086, 0x2415) => "82801AA AC'97 Audio Controller",
            (0x8086, 0xA7A0) => "Raptor Lake-P [Iris Xe Graphics]",
            (0x80EE, 0xBEEF) => "VirtualBox Graphics Adapter (VBE/BGA)",
            (0x80EE, 0xCAFE) => "VirtualBox Guest Service Device (VMMDev)",
            (0x106B, 0x003F) => "KeyLargo/Intrepid USB OHCI Controller",
            _ => dev.subclass_name(),
        };

        out.push_str(&format!(
            "{} {}: {} {} [{:04x}:{:04x}] (rev {:02x})\n",
            dev.addr.to_string(),
            dev.subclass_name(),
            vendor_name,
            dev_desc,
            dev.vendor_id,
            dev.device_id,
            dev.revision_id
        ));

        if verbose {
            out.push_str(&format!(
                "        Class: {:02x}:{:02x} ({}) | ProgIF: {:02x} ({}) | IRQ: {}\n",
                dev.class_code,
                dev.subclass,
                dev.class_name(),
                dev.prog_if,
                dev.prog_if_name(),
                if dev.interrupt_line == 0xFF || dev.interrupt_line == 0 {
                    String::from("none")
                } else {
                    format!("{}", dev.interrupt_line)
                }
            ));
            for (idx, bar_opt) in dev.bars.iter().enumerate() {
                if let Some(bar) = bar_opt {
                    match bar {
                        Bar::Memory { address, size, prefetchable, is_64bit } => {
                            out.push_str(&format!(
                                "        BAR{}: Memory at {:#010x} ({}-bit, {}) [size={} KiB]\n",
                                idx,
                                address,
                                if *is_64bit { 64 } else { 32 },
                                if *prefetchable { "prefetchable" } else { "non-prefetchable" },
                                size / 1024
                            ));
                        }
                        Bar::Io { port, size } => {
                            out.push_str(&format!(
                                "        BAR{}: I/O ports at {:#06x} [size={}]\n",
                                idx, port, size
                            ));
                        }
                    }
                }
            }
        }
    }

    out
}

/// Generates a comprehensive, unified hardware inventory report (`hwinfo`).
pub fn format_hwinfo() -> String {
    let mut out = String::new();
    let cpu = CpuInfo::query();

    out.push_str("========================================================================\n");
    out.push_str("            INTEGRATE OS — UNIFIED HARDWARE INVENTORY (hwinfo)          \n");
    out.push_str("========================================================================\n");

    // 1. CPU (Live CPUID)
    out.push_str(&format!(
        "[CPU]        Vendor: {} | Family: {}, Model: {}, Stepping: {}\n",
        cpu.vendor_str(),
        cpu.family,
        cpu.model,
        cpu.stepping
    ));
    out.push_str(&format!(
        "             Features: 64-Bit LongMode={}, NX={}, APIC={}, SSE2={}\n",
        cpu.has_long_mode, cpu.has_nx, cpu.has_apic, cpu.has_sse2
    ));

    // 2. Laptop Hardware Platform Profile
    {
        let laptop = LAPTOP_MANAGER.lock();
        let prof = laptop.profile();
        let bat = laptop.battery();
        let therm = laptop.thermal();
        out.push_str(&format!(
            "[PLATFORM]   Host Machine : {} | CPU: {}\n",
            prof.device_name, prof.cpu_model
        ));
        out.push_str(&format!(
            "             Topology     : {} Cores ({}P + {}E), {} Threads | Installed RAM: {} GB\n",
            prof.p_cores + prof.e_cores,
            prof.p_cores,
            prof.e_cores,
            prof.total_threads,
            prof.ram_installed_gb
        ));
        out.push_str(&format!(
            "             Host GPU     : {}\n",
            prof.gpu_name
        ));
        out.push_str(&format!(
            "             ACPI Sensors : Battery {}% ({}, {} mV) | CPU Temp: {} C\n",
            bat.percentage, bat.state, bat.voltage_mv, therm.temperature_celsius
        ));
    }

    // 3. Memory (Live Physical Frame Allocator & Kernel Heap)
    {
        let fa = FRAME_ALLOCATOR.lock();
        let total_mib = (fa.total_frames() * PAGE_SIZE) / (1024 * 1024);
        let free_mib = (fa.free_frames() * PAGE_SIZE) / (1024 * 1024);
        let used_mib = total_mib.saturating_sub(free_mib);
        drop(fa);

        let heap = ALLOCATOR.0.lock();
        let heap_used_kib = heap.allocated_bytes() / 1024;
        let heap_total_kib = heap.total_bytes() / 1024;
        drop(heap);

        out.push_str(&format!(
            "[MEMORY]     Guest RAM    : {} MiB Total ({} MiB Used, {} MiB Free)\n",
            total_mib, used_mib, free_mib
        ));
        out.push_str(&format!(
            "             Kernel Heap  : {} KiB Allocated / {} KiB Total\n",
            heap_used_kib, heap_total_kib
        ));
    }

    // 4. PCI / PCIe Bus Devices (Live Probed)
    {
        let pci = PCI_MANAGER.lock();
        out.push_str(&format!(
            "[PCI/PCIe]   Enumerated {} Hardware Functions across {} Buses:\n",
            pci.devices.len(),
            pci.buses_scanned
        ));
        for dev in &pci.devices {
            let bar0_str = match dev.bars[0] {
                Some(Bar::Memory { address, .. }) => format!("MMIO {:#010x}", address),
                Some(Bar::Io { port, .. }) => format!("IO {:#06x}", port),
                None => String::from("No BAR0"),
            };
            out.push_str(&format!(
                "             - [{}] ID {:04x}:{:04x} | {} ({}) | {} | IRQ {}\n",
                dev.addr.to_string(),
                dev.vendor_id,
                dev.device_id,
                dev.subclass_name(),
                dev.class_name(),
                bar0_str,
                dev.interrupt_line
            ));
        }
    }

    // 5. Storage Controllers & SSDs
    {
        let ssd_summary = crate::storage::ssd::format_storage_controllers_summary();
        for line in ssd_summary.lines() {
            out.push_str(&format!("[STORAGE]    {}\n", line));
        }
    }

    // 6. Network & WiFi Controllers
    {
        let stack = NETWORK_STACK.lock();
        let mac = stack.e1000.mac_address();
        let cfg = &stack.config;
        out.push_str(&format!(
            "[NETWORK]    LAN Interface: eth0 (Intel E1000 8086:100e) | MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n",
            mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]
        ));
        out.push_str(&format!(
            "             IPv4 Address : {}.{}.{}.{} | Gateway: {}.{}.{}.{} | Netmask: {}.{}.{}.{}\n",
            cfg.ip.0[0], cfg.ip.0[1], cfg.ip.0[2], cfg.ip.0[3],
            cfg.gateway.0[0], cfg.gateway.0[1], cfg.gateway.0[2], cfg.gateway.0[3],
            cfg.subnet_mask.0[0], cfg.subnet_mask.0[1], cfg.subnet_mask.0[2], cfg.subnet_mask.0[3]
        ));
        drop(stack);

        let wifi = WIFI_MANAGER.lock();
        let ssid_str = wifi.connected_ssid().unwrap_or("Not Connected");
        let wip = wifi.connected_ip().unwrap_or(crate::net::types::Ipv4Address::new(0, 0, 0, 0));
        out.push_str(&format!(
            "[WIFI]       802.11 Radio : wlan0 (Intel Wi-Fi 6 AX201) | State: {} | SSID: {}\n",
            wifi.state_str(),
            ssid_str
        ));
        out.push_str(&format!(
            "             Wireless IP  : {}.{}.{}.{} | Nearby APs Discovered: {}\n",
            wip.0[0], wip.0[1], wip.0[2], wip.0[3],
            wifi.scanned_aps().len()
        ));
    }

    // 7. Display, Input & Timers
    {
        let (w, h, bpp, lfb) = crate::graphics::GRAPHICS
            .lock()
            .as_ref()
            .map(|g| (g.mode.width, g.mode.height, g.mode.bpp, g.mode.phys_addr))
            .unwrap_or((1024, 768, 32, 0xE000_0000));
        out.push_str(&format!(
            "[DISPLAY]    Active Mode  : {}x{}x{}bpp Linear Framebuffer @ {:#010x}\n",
            w, h, bpp, lfb
        ));
        let (mx, my) = crate::input::cursor_position();
        out.push_str(&format!(
            "[INPUT]      Keyboard: PS/2 Set-1 (IRQ 1) | Mouse: PS/2 Auxiliary (IRQ 12) @ ({}, {})\n",
            mx, my
        ));
        let dev_count = DEVICE_MANAGER.lock().count();
        out.push_str(&format!(
            "[TIMERS]     8254 PIT @ 100 Hz (Ticks: {}) | Registered Drivers: {}\n",
            pit::get_ticks(),
            dev_count
        ));
    }

    out.push_str("========================================================================\n");
    out
}
