//! Integrate OS — Subsystem Registry & Lifecycle Metadata
//!
//! Tracks operational status, architectural layers, and dependency graphs
//! across all 32 kernel subsystems (Phases 1 through 32).

use alloc::vec::Vec;
use core::fmt;

/// Unique identifier for each kernel subsystem (Phases 1..=32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SubsystemId {
    Boot = 1,
    CpuArch = 2,
    Exceptions = 3,
    Interrupts = 4,
    PhysicalMemory = 5,
    VirtualMemory = 6,
    KernelHeap = 7,
    DriverArchitecture = 8,
    FramebufferGraphics = 9,
    UserMode = 10,
    Processes = 11,
    SystemCalls = 12,
    Scheduler = 13,
    Storage = 14,
    VirtualFilesystem = 15,
    InputSubsystem = 16,
    WindowManager = 17,
    Compositor = 18,
    GuiToolkit = 19,
    DesktopEnvironment = 20,
    ApplicationFramework = 21,
    DesktopApplications = 22,
    Networking = 23,
    PciBus = 24,
    UsbSubsystem = 25,
    GpuFoundation = 26,
    AudioSubsystem = 27,
    TimeSubsystem = 28,
    AcpiPower = 29,
    SecurityIsolation = 30,
    Synchronization = 31,
    DebuggingReliability = 32,
}

impl SubsystemId {
    /// Returns the Phase number associated with this subsystem.
    pub const fn phase(self) -> u8 {
        self as u8
    }

    /// Returns the human-readable display name of the subsystem.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Boot => "GRUB + Bootloader",
            Self::CpuArch => "x86_64 CPU Architecture",
            Self::Exceptions => "IDT + CPU Exceptions",
            Self::Interrupts => "Hardware Interrupts (PIC/PIT)",
            Self::PhysicalMemory => "Physical Memory (PMM)",
            Self::VirtualMemory => "Virtual Memory & Paging (VMM)",
            Self::KernelHeap => "Kernel Dynamic Heap",
            Self::DriverArchitecture => "Driver Framework & Registry",
            Self::FramebufferGraphics => "Linear Framebuffer & Graphics",
            Self::UserMode => "User Mode (Ring 3)",
            Self::Processes => "Process Table & PCB",
            Self::SystemCalls => "System Call ABI Dispatcher",
            Self::Scheduler => "Preemptive Scheduler",
            Self::Storage => "ATA / IDE Block Storage",
            Self::VirtualFilesystem => "Virtual Filesystem (VFS)",
            Self::InputSubsystem => "Input (Keyboard/Mouse)",
            Self::WindowManager => "Window Manager (WM)",
            Self::Compositor => "Compositor & Layer Effects",
            Self::GuiToolkit => "GUI Toolkit & Widgets",
            Self::DesktopEnvironment => "Desktop Shell & Start Menu",
            Self::ApplicationFramework => "Application Framework Runtime",
            Self::DesktopApplications => "Desktop Application Suite",
            Self::Networking => "Network Stack (Ethernet/IP/TCP)",
            Self::PciBus => "PCI / PCIe Bus Enumeration",
            Self::UsbSubsystem => "USB Host & Device Drivers",
            Self::GpuFoundation => "GPU Foundation & 2D Engine",
            Self::AudioSubsystem => "Audio Subsystem & Synthesizer",
            Self::TimeSubsystem => "Time & RTC Subsystem",
            Self::AcpiPower => "ACPI & Power Management",
            Self::SecurityIsolation => "Security & Isolation Subsystem",
            Self::Synchronization => "Kernel Synchronization",
            Self::DebuggingReliability => "Debugging & Reliability",
        }
    }

    /// Returns the architectural category of the subsystem.
    pub const fn category(self) -> SubsystemCategory {
        match self {
            Self::Boot | Self::CpuArch | Self::Exceptions | Self::Interrupts => {
                SubsystemCategory::Core
            }
            Self::PhysicalMemory | Self::VirtualMemory | Self::KernelHeap => {
                SubsystemCategory::Memory
            }
            Self::UserMode | Self::Processes | Self::SystemCalls | Self::Scheduler | Self::Synchronization => {
                SubsystemCategory::Execution
            }
            Self::DriverArchitecture | Self::PciBus | Self::UsbSubsystem | Self::GpuFoundation
            | Self::AudioSubsystem | Self::TimeSubsystem | Self::AcpiPower => {
                SubsystemCategory::Hardware
            }
            Self::Storage | Self::VirtualFilesystem => SubsystemCategory::StorageFs,
            Self::InputSubsystem
            | Self::FramebufferGraphics
            | Self::WindowManager
            | Self::Compositor
            | Self::GuiToolkit
            | Self::DesktopEnvironment
            | Self::ApplicationFramework
            | Self::DesktopApplications => SubsystemCategory::GraphicsUi,
            Self::Networking => SubsystemCategory::Connectivity,
            Self::SecurityIsolation | Self::DebuggingReliability => SubsystemCategory::SecurityDebug,
        }
    }

    /// Returns a list of direct subsystem dependencies required by this subsystem.
    pub const fn direct_dependencies(self) -> &'static [SubsystemId] {
        match self {
            Self::Boot => &[],
            Self::CpuArch => &[Self::Boot],
            Self::Exceptions => &[Self::CpuArch],
            Self::Interrupts => &[Self::Exceptions],
            Self::PhysicalMemory => &[Self::Boot],
            Self::VirtualMemory => &[Self::PhysicalMemory, Self::CpuArch],
            Self::KernelHeap => &[Self::VirtualMemory],
            Self::DriverArchitecture => &[Self::KernelHeap],
            Self::FramebufferGraphics => &[Self::DriverArchitecture],
            Self::UserMode => &[Self::VirtualMemory, Self::CpuArch],
            Self::Processes => &[Self::KernelHeap, Self::UserMode],
            Self::SystemCalls => &[Self::Processes, Self::UserMode],
            Self::Scheduler => &[Self::Processes, Self::Interrupts],
            Self::Storage => &[Self::DriverArchitecture, Self::Interrupts],
            Self::VirtualFilesystem => &[Self::KernelHeap, Self::Storage],
            Self::InputSubsystem => &[Self::DriverArchitecture, Self::VirtualFilesystem],
            Self::WindowManager => &[Self::FramebufferGraphics, Self::InputSubsystem],
            Self::Compositor => &[Self::WindowManager],
            Self::GuiToolkit => &[Self::Compositor],
            Self::DesktopEnvironment => &[Self::GuiToolkit],
            Self::ApplicationFramework => &[Self::DesktopEnvironment],
            Self::DesktopApplications => &[Self::ApplicationFramework],
            Self::Networking => &[Self::DriverArchitecture, Self::VirtualFilesystem],
            Self::PciBus => &[Self::DriverArchitecture],
            Self::UsbSubsystem => &[Self::PciBus, Self::DriverArchitecture],
            Self::GpuFoundation => &[Self::PciBus, Self::FramebufferGraphics],
            Self::AudioSubsystem => &[Self::PciBus, Self::DriverArchitecture],
            Self::TimeSubsystem => &[Self::Interrupts],
            Self::AcpiPower => &[Self::DriverArchitecture],
            Self::SecurityIsolation => &[Self::Processes, Self::VirtualMemory],
            Self::Synchronization => &[Self::Scheduler],
            Self::DebuggingReliability => &[Self::CpuArch, Self::VirtualMemory],
        }
    }
}

/// Architectural grouping of kernel subsystems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemCategory {
    Core,
    Memory,
    Execution,
    Hardware,
    StorageFs,
    GraphicsUi,
    Connectivity,
    SecurityDebug,
}

impl SubsystemCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "Core Arch",
            Self::Memory => "Memory",
            Self::Execution => "Execution",
            Self::Hardware => "Hardware/Drivers",
            Self::StorageFs => "Storage & VFS",
            Self::GraphicsUi => "Graphics & UI",
            Self::Connectivity => "Connectivity",
            Self::SecurityDebug => "Security & Debug",
        }
    }
}

/// Current operational health state of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemState {
    Operational,
    Degraded,
    Initializing,
    Uninitialized,
    Failed,
}

impl SubsystemState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Operational => "OPERATIONAL",
            Self::Degraded => "DEGRADED",
            Self::Initializing => "INITIALIZING",
            Self::Uninitialized => "UNINITIALIZED",
            Self::Failed => "FAILED",
        }
    }

    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Operational | Self::Degraded)
    }
}

impl fmt::Display for SubsystemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Comprehensive metadata and live state record for a kernel subsystem.
#[derive(Debug, Clone)]
pub struct SubsystemInfo {
    pub id: SubsystemId,
    pub name: &'static str,
    pub phase: u8,
    pub category: SubsystemCategory,
    pub description: &'static str,
    pub state: SubsystemState,
    pub dependencies: &'static [SubsystemId],
}

impl SubsystemInfo {
    pub fn new(
        id: SubsystemId,
        description: &'static str,
        state: SubsystemState,
    ) -> Self {
        Self {
            id,
            name: id.name(),
            phase: id.phase(),
            category: id.category(),
            description,
            state,
            dependencies: id.direct_dependencies(),
        }
    }
}

/// Central catalog and health tracker for all 32 subsystems.
pub struct SubsystemRegistry {
    entries: Vec<SubsystemInfo>,
}

impl SubsystemRegistry {
    /// Builds the registry pre-populated with all 32 subsystems.
    pub fn new() -> Self {
        let mut reg = Self {
            entries: Vec::with_capacity(32),
        };
        reg.init_defaults();
        reg.refresh_states();
        reg
    }

    fn init_defaults(&mut self) {
        let defs = [
            (SubsystemId::Boot, "GRUB Multiboot1 specification & handoff protocol"),
            (SubsystemId::CpuArch, "x86_64 GDT, TSS, Privilege Rings & Control Registers"),
            (SubsystemId::Exceptions, "Interrupt Descriptor Table (IDT) & CPU Fault Handlers"),
            (SubsystemId::Interrupts, "8259 PIC remapping & 8254 PIT 100 Hz timer interrupt"),
            (SubsystemId::PhysicalMemory, "E820 BIOS memory map parsing & frame bitmap allocator"),
            (SubsystemId::VirtualMemory, "4-Level paging (PML4), dynamic mapping & TLB flushing"),
            (SubsystemId::KernelHeap, "GlobalAlloc allocator supporting Box, Vec, String collections"),
            (SubsystemId::DriverArchitecture, "Unified Device Manager & hardware driver registry"),
            (SubsystemId::FramebufferGraphics, "Linear framebuffer rendering, 2D primitives & font engine"),
            (SubsystemId::UserMode, "Ring 3 separation, iretq context switch & user stack isolation"),
            (SubsystemId::Processes, "Process Control Block (PCB), PID management & lifecycle states"),
            (SubsystemId::SystemCalls, "System call ABI dispatcher, int 0x80 gate & pointer validation"),
            (SubsystemId::Scheduler, "Priority run queues, cooperative yield & preemptive timeslicing"),
            (SubsystemId::Storage, "IDE / ATA PIO controller driver & RamDisk block device"),
            (SubsystemId::VirtualFilesystem, "VFS hierarchy, Inodes, RamFS, DevFS & file descriptor table"),
            (SubsystemId::InputSubsystem, "PS/2 keyboard decoder, mouse packet parser & event queue"),
            (SubsystemId::WindowManager, "Z-ordered stacking, window decorations, focus & dragging"),
            (SubsystemId::Compositor, "Multi-layer composition, damage tracking & drop shadow rendering"),
            (SubsystemId::GuiToolkit, "Widget library (Button, Label, CheckBox, Layout Containers)"),
            (SubsystemId::DesktopEnvironment, "Integrate OS desktop wallpaper, icons & Start Menu"),
            (SubsystemId::ApplicationFramework, "App lifecycle runtime, event bus & system clipboard"),
            (SubsystemId::DesktopApplications, "Terminal, File Manager, System Monitor, Editor & Settings"),
            (SubsystemId::Networking, "Ethernet II, ARP, IPv4, ICMP, UDP, TCP & socket interface"),
            (SubsystemId::PciBus, "PCI 2.3 / PCIe bus enumeration, config mechanism #1 & BARs"),
            (SubsystemId::UsbSubsystem, "Host controller discovery, descriptor parsing & HID/MSC drivers"),
            (SubsystemId::GpuFoundation, "PCI display adapter identification & 2D GPU acceleration"),
            (SubsystemId::AudioSubsystem, "PCI audio controller detection, PCM mixer & waveform synth"),
            (SubsystemId::TimeSubsystem, "CMOS RTC, Gregorian calendar, monotonic uptime & timers"),
            (SubsystemId::AcpiPower, "ACPI RSDP/RSDT/XSDT discovery, FADT, MADT & power management"),
            (SubsystemId::SecurityIsolation, "UserPtr validation, capability enforcement & audit logging"),
            (SubsystemId::Synchronization, "IrqSpinlocks, RwLocks, Mutexes, Semaphores & WaitQueues"),
            (SubsystemId::DebuggingReliability, "Structured logger, log ring buffer, stack unwind & errors"),
        ];

        for (id, desc) in defs {
            self.entries.push(SubsystemInfo::new(id, desc, SubsystemState::Uninitialized));
        }
    }

    /// Evaluates live subsystem states against real kernel hardware and memory structures.
    pub fn refresh_states(&mut self) {
        for info in self.entries.iter_mut() {
            info.state = match info.id {
                SubsystemId::Boot => SubsystemState::Operational,
                SubsystemId::CpuArch => {
                    let cr3 = crate::memory_layout::read_cr3();
                    if cr3 != 0 { SubsystemState::Operational } else { SubsystemState::Failed }
                }
                SubsystemId::Exceptions => SubsystemState::Operational,
                SubsystemId::Interrupts => {
                    let ticks = crate::arch::x86_64::pit::get_ticks();
                    if ticks > 0 { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::PhysicalMemory => {
                    let allocator = crate::memory::frame_allocator::FRAME_ALLOCATOR.lock();
                    if allocator.total_frames() > 0 { SubsystemState::Operational } else { SubsystemState::Failed }
                }
                SubsystemId::VirtualMemory => {
                    let cr3 = crate::memory_layout::read_cr3();
                    if cr3 != 0 { SubsystemState::Operational } else { SubsystemState::Failed }
                }
                SubsystemId::KernelHeap => {
                    let stats = crate::memory::heap::ALLOCATOR.0.lock();
                    if stats.total_bytes() > 0 { SubsystemState::Operational } else { SubsystemState::Failed }
                }
                SubsystemId::DriverArchitecture => {
                    let dev_mgr = crate::drivers::manager::DEVICE_MANAGER.lock();
                    if dev_mgr.count() > 0 { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::FramebufferGraphics => SubsystemState::Operational,
                SubsystemId::UserMode => SubsystemState::Operational,
                SubsystemId::Processes => {
                    let pm = crate::process::manager::PROCESS_MANAGER.lock();
                    if pm.count() > 0 { SubsystemState::Operational } else { SubsystemState::Failed }
                }
                SubsystemId::SystemCalls => SubsystemState::Operational,
                SubsystemId::Scheduler => SubsystemState::Operational,
                SubsystemId::Storage => {
                    if crate::storage::is_storage_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::VirtualFilesystem => {
                    if crate::fs::VFS.lock().is_some() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::InputSubsystem => SubsystemState::Operational,
                SubsystemId::WindowManager => {
                    if crate::wm::WINDOW_MANAGER.lock().is_some() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::Compositor => {
                    if crate::compositor::COMPOSITOR.lock().is_some() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::GuiToolkit => SubsystemState::Operational,
                SubsystemId::DesktopEnvironment => {
                    if crate::desktop::DESKTOP_SHELL.lock().is_some() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::ApplicationFramework => {
                    if crate::framework::APP_MANAGER.lock().is_some() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::DesktopApplications => SubsystemState::Operational,
                SubsystemId::Networking => {
                    if crate::net::is_network_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::PciBus => {
                    if crate::pci::is_pci_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::UsbSubsystem => {
                    if crate::usb::is_usb_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::GpuFoundation => {
                    if crate::gpu::is_gpu_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::AudioSubsystem => {
                    if crate::audio::is_audio_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::TimeSubsystem => {
                    if crate::time::is_time_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::AcpiPower => {
                    if crate::acpi::is_acpi_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::SecurityIsolation => {
                    if crate::security::is_security_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::Synchronization => {
                    if crate::sync::is_sync_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
                SubsystemId::DebuggingReliability => {
                    if crate::debug::is_debug_initialized() { SubsystemState::Operational } else { SubsystemState::Degraded }
                }
            };
        }
    }

    /// Returns the total count of registered subsystems (32).
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the count of fully operational subsystems.
    pub fn operational_count(&self) -> usize {
        self.entries.iter().filter(|s| s.state == SubsystemState::Operational).count()
    }

    /// Looks up a subsystem record by its ID.
    pub fn get(&self, id: SubsystemId) -> Option<&SubsystemInfo> {
        self.entries.iter().find(|s| s.id == id)
    }

    /// Returns a slice of all registered subsystems.
    pub fn all(&self) -> &[SubsystemInfo] {
        &self.entries
    }

    /// Verifies that all 32 subsystems are healthy (Operational or Degraded).
    pub fn verify_all_healthy(&self) -> bool {
        self.entries.iter().all(|s| s.state.is_healthy())
    }

    /// Verifies that all direct dependencies of every subsystem are satisfied and healthy.
    pub fn verify_dependencies(&self) -> Result<(), (SubsystemId, SubsystemId)> {
        for sub in &self.entries {
            for &dep_id in sub.dependencies {
                if let Some(dep) = self.get(dep_id) {
                    if !dep.state.is_healthy() {
                        return Err((sub.id, dep_id));
                    }
                } else {
                    return Err((sub.id, dep_id));
                }
            }
        }
        Ok(())
    }
}
