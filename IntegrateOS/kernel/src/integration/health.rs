//! Integrate OS — Global System Health & Telemetry Collector
//!
//! Provides comprehensive snapshotting of kernel physical/virtual memory,
//! process scheduling, driver registry, storage, networking, security, and time.

use alloc::string::String;
use crate::memory::frame_allocator::PAGE_SIZE;

/// Comprehensive snapshot of real-time kernel telemetry.
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    // Memory
    pub total_frames: usize,
    pub free_frames: usize,
    pub used_frames: usize,
    pub total_ram_mib: usize,
    pub free_ram_mib: usize,
    pub heap_total_bytes: usize,
    pub heap_allocated_bytes: usize,
    pub heap_free_bytes: usize,

    // Scheduling
    pub total_processes: usize,
    pub current_pid: u64,
    pub context_switches: u64,
    pub preemptions: u64,
    pub yields: u64,

    // Hardware & Drivers
    pub registered_drivers: usize,
    pub supported_drivers: usize,
    pub pci_devices: usize,
    pub usb_devices: usize,
    pub acpi_tables: usize,

    // Storage & Filesystem
    pub storage_devices: usize,
    pub root_dir_entries: usize,

    // Network
    pub network_active: bool,
    pub ip_address: [u8; 4],
    pub udp_sockets: usize,
    pub tcp_connections: usize,

    // Time & Uptime
    pub uptime_ms: u64,
    pub wall_clock: String,

    // Security & Diagnostics
    pub audit_events: usize,
    pub security_contexts: usize,
    pub diagnostic_log_entries: usize,
    pub cpu_faults: usize,
}

impl SystemHealthReport {
    /// Gathers live telemetry from all active kernel subsystems.
    pub fn collect() -> Self {
        // 1. Memory telemetry
        let (total_frames, free_frames) = {
            let fa = crate::memory::frame_allocator::FRAME_ALLOCATOR.lock();
            (fa.total_frames(), fa.free_frames())
        };
        let used_frames = total_frames.saturating_sub(free_frames);
        let total_ram_mib = (total_frames * PAGE_SIZE) / (1024 * 1024);
        let free_ram_mib = (free_frames * PAGE_SIZE) / (1024 * 1024);

        let (heap_total, heap_allocated, heap_free) = {
            let stats = crate::memory::heap::ALLOCATOR.0.lock();
            (stats.total_bytes(), stats.allocated_bytes(), stats.free_bytes())
        };

        // 2. Process & Scheduler telemetry
        let total_processes = crate::process::manager::PROCESS_MANAGER.lock().count();
        let (context_switches, preemptions, yields) = crate::scheduler::stats();
        let current_pid = crate::scheduler::SCHEDULER.lock().current_pid().as_u64();

        // 3. Hardware & Drivers telemetry
        let (registered_drivers, supported_drivers) = {
            let dev_mgr = crate::drivers::manager::DEVICE_MANAGER.lock();
            (
                dev_mgr.count(),
                dev_mgr.count_by_status(crate::drivers::traits::HardwareSupportStatus::Supported),
            )
        };

        let pci_devices = crate::pci::manager::PCI_MANAGER.lock().count();
        let usb_devices = crate::usb::manager::USB_MANAGER.lock().device_count();
        let acpi_tables = {
            let lock = crate::acpi::manager::ACPI_MANAGER.lock();
            lock.as_ref().map(|a| a.tables().entries().len()).unwrap_or(0)
        };

        // 4. Storage & Filesystem telemetry
        let storage_devices = {
            let dev_mgr = crate::drivers::manager::DEVICE_MANAGER.lock();
            dev_mgr.find_by_category(crate::drivers::device::DeviceCategory::Storage).len()
        };
        let root_dir_entries = crate::fs::readdir("/").map(|entries| entries.len()).unwrap_or(0);

        // 5. Network telemetry
        let (network_active, ip_address, udp_sockets, tcp_connections) = {
            if crate::net::is_network_initialized() {
                let stack = crate::net::stack::NETWORK_STACK.lock();
                (
                    true,
                    stack.config.ip.0,
                    stack.udp_sockets.sockets.len(),
                    stack.tcp_connections.len(),
                )
            } else {
                (false, [0, 0, 0, 0], 0, 0)
            }
        };

        // 6. Time telemetry
        let uptime_ms = crate::time::uptime_ms();
        let wall_clock = crate::time::format_now_iso8601();

        // 7. Security & Debug telemetry
        let (audit_events, security_contexts) = {
            crate::security::with_security(|sm| (sm.audit_log().event_count(), sm.contexts_count()))
                .unwrap_or((0, 0))
        };

        let (diagnostic_log_entries, cpu_faults) = {
            crate::debug::with_debug(|dm| (dm.ring_buffer().len(), dm.fault_count()))
                .unwrap_or((0, 0))
        };

        Self {
            total_frames,
            free_frames,
            used_frames,
            total_ram_mib,
            free_ram_mib,
            heap_total_bytes: heap_total,
            heap_allocated_bytes: heap_allocated,
            heap_free_bytes: heap_free,
            total_processes,
            current_pid,
            context_switches,
            preemptions,
            yields,
            registered_drivers,
            supported_drivers,
            pci_devices,
            usb_devices,
            acpi_tables,
            storage_devices,
            root_dir_entries,
            network_active,
            ip_address,
            udp_sockets,
            tcp_connections,
            uptime_ms,
            wall_clock,
            audit_events,
            security_contexts,
            diagnostic_log_entries,
            cpu_faults,
        }
    }

    /// Emits structured telemetry summary to the serial logger.
    pub fn log_summary(&self) {
        crate::klog_info!("---------------- SYSTEM HEALTH & TELEMETRY ----------------");
        crate::klog_info!(
            "  Memory       : {}/{} MiB Free ({} frames used, {} free)",
            self.free_ram_mib,
            self.total_ram_mib,
            self.used_frames,
            self.free_frames
        );
        crate::klog_info!(
            "  Kernel Heap  : {} KiB Total, {} KiB Alloc, {} KiB Free",
            self.heap_total_bytes / 1024,
            self.heap_allocated_bytes / 1024,
            self.heap_free_bytes / 1024
        );
        crate::klog_info!(
            "  Multitasking : {} Tasks, Current PID={}, Sw={} Pr={} Yld={}",
            self.total_processes,
            self.current_pid,
            self.context_switches,
            self.preemptions,
            self.yields
        );
        crate::klog_info!(
            "  Hardware     : Drivers={}/{} Supported, PCI={}, USB={}, ACPI={}",
            self.supported_drivers,
            self.registered_drivers,
            self.pci_devices,
            self.usb_devices,
            self.acpi_tables
        );
        crate::klog_info!(
            "  Filesystem   : Storage Devices={}, VFS Root Entries={}",
            self.storage_devices,
            self.root_dir_entries
        );
        crate::klog_info!(
            "  Network      : Active={}, IP={}.{}.{}.{}, UDP={}, TCP={}",
            self.network_active,
            self.ip_address[0],
            self.ip_address[1],
            self.ip_address[2],
            self.ip_address[3],
            self.udp_sockets,
            self.tcp_connections
        );
        crate::klog_info!(
            "  Security/Rel : Audit Events={}, Contexts={}, RingBuf={}, Faults={}",
            self.audit_events,
            self.security_contexts,
            self.diagnostic_log_entries,
            self.cpu_faults
        );
        crate::klog_info!(
            "  Clock/Uptime : Wall Clock '{}', Uptime {} ms",
            self.wall_clock.as_str(),
            self.uptime_ms
        );
        crate::klog_info!("-----------------------------------------------------------");
    }
}
