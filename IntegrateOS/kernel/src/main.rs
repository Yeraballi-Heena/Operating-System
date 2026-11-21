//! Integrate OS — 64-Bit Bare-Metal Rust Kernel
//!
//! Phase 1: GRUB + Rust Kernel Boot
//! Phase 2: x86_64 CPU Architecture (GDT, TSS, Privilege Rings, Control Registers)
//! Phase 3: IDT + CPU Exceptions
//! Phase 4: Hardware Interrupts (8259 PIC, 8254 PIT, PS/2 Keyboard, IrqSpinlock)
//! Phase 5: Physical Memory Management (E820 Memory Map, Physical Frame Allocator)
//! Phase 6: Paging + Virtual Memory (4-Level Page Tables, Dynamic Mapping, Translation, TLB Invalidation)
//! Phase 7: Kernel Heap + Rust Alloc (GlobalAlloc, Box, Vec, String, Dynamic Collections)
//! Phase 8: Driver Architecture (Device Registry, Hardware Status Matrix, Driver Traits)
//! Phase 9: Framebuffer + Graphics (Linear Framebuffer, 2D Primitives, Fonts, Clipping, Double Buffering, Milestone G)
//! Phase 10: User Mode (Ring 3 Privilege Separation, User Stack & Page Table Isolation, int 0x80 Syscalls)
//! Phase 11: Processes (Process Control Block, PID Allocation, Context Switch, Lifecycle State Machine)

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod acpi;
pub mod arch;
pub mod apps;
pub mod audio;
pub mod boot;
pub mod compositor;
pub mod debug;
pub mod desktop;
pub mod drivers;
pub mod fs;
pub mod framework;
pub mod gpu;
pub mod graphics;
pub mod gui;
pub mod input;
pub mod integration;
pub mod logger;
pub mod memory;
pub mod memory_layout;
pub mod net;
pub mod pci;
pub mod pkg;
pub mod process;
pub mod research;
pub mod scheduler;
pub mod security;
pub mod serial;
pub mod storage;
pub mod sync;
pub mod syscall;
pub mod time;
pub mod usb;
pub mod user;
pub mod vga;
pub mod wm;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::arch::x86_64::interrupts;
use crate::arch::x86_64::pit;
use crate::boot::{MultibootInfo, MULTIBOOT_BOOTLOADER_MAGIC};
use crate::drivers::manager::{self, DEVICE_MANAGER};
use crate::drivers::traits::HardwareSupportStatus;
use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::heap::{self, ALLOCATOR};
use crate::memory::mmap;
use crate::memory::paging;
use crate::memory_layout::{read_cr3, read_rsp, KernelMemoryLayout};
use crate::vga::Color;
use core::panic::PanicInfo;

/// Primary 64-bit Rust kernel entry point invoked from `long_mode_start` (`boot.rs`).
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_magic: u32, multiboot_info_addr: u64) -> ! {
    // 1. Initialize COM1 Serial UART (0x3F8) and VGA Text Console (0xB8000)
    serial::init();
    vga::init();

    // 2. Verify GRUB Multiboot1 handoff magic (0x2BADB002)
    if multiboot_magic != MULTIBOOT_BOOTLOADER_MAGIC {
        panic!(
            "Invalid Multiboot magic: expected {:#010x}, got {:#010x}",
            MULTIBOOT_BOOTLOADER_MAGIC, multiboot_magic
        );
    }

    // 3. Emit mandatory boot banner to both VGA screen and COM1 serial
    vga::set_color(Color::White, Color::Black);
    kprintln!("INTEGRATE OS");
    kprintln!("--------------");
    vga::set_color(Color::LightGreen, Color::Black);
    kprintln!("GRUB BOOT SUCCESS");
    kprintln!("64-BIT KERNEL STARTED");
    kprintln!("KERNEL INITIALIZED");
    vga::set_color(Color::LightGray, Color::Black);
    kprintln!();

    // 4. Inspect and log Multiboot handoff metadata
    // SAFETY: multiboot_info_addr is provided by GRUB in identity-mapped low memory.
    let mb_info = unsafe { MultibootInfo::from_phys_addr(multiboot_info_addr) };
    if let Some(info) = mb_info {
        klog_ok!(
            "Bootloader handoff verified: {} (magic={:#010x}, info={:#010x})",
            info.bootloader_name(),
            multiboot_magic,
            multiboot_info_addr
        );
    } else {
        klog_ok!(
            "Bootloader handoff verified (magic={:#010x})",
            multiboot_magic
        );
    }

    // 5. Verify 64-bit CPU mode, PML4 page table base (CR3), and kernel stack
    let layout = KernelMemoryLayout::get();
    let rsp = read_rsp();
    let cr3 = read_cr3();

    if !layout.is_rsp_within_stack(rsp) {
        panic!(
            "Kernel RSP ({:#x}) is outside stack bounds [{:#x} .. {:#x}]",
            rsp, layout.stack_bottom, layout.stack_top
        );
    }

    klog_ok!(
        "CPU Mode: 64-bit Long Mode active (CR3={:#010x}, RSP={:#010x})",
        cr3,
        rsp
    );
    klog_info!(
        "Kernel Layout : [{:#010x} .. {:#010x}] ({} KiB)",
        layout.kernel_start,
        layout.kernel_end,
        layout.total_size() / 1024
    );
    klog_ok!(
        "Kernel Stack  : [{:#010x} .. {:#010x}] ({} KiB, 16-byte aligned)",
        layout.stack_bottom,
        layout.stack_top,
        layout.stack_size() / 1024
    );

    // 6. Phase 2 & 3: Initialize and verify x86_64 CPU Architecture Layer (GDT, TSS, IDT)
    arch::x86_64::init();
    arch::x86_64::log_architecture_state();

    // 7. Phase 3: Run intentional safe CPU exception self-tests (#BP, #DE, #UD)
    arch::x86_64::exceptions::run_exception_self_tests();

    // 8. Phase 4: Initialize Hardware Interrupts (8259 PIC, 8254 PIT @ 100 Hz, PS/2 Keyboard)
    interrupts::init();
    klog_ok!("8259 PIC Remapped: Master=32..39, Slave=40..47");
    klog_ok!("8254 PIT Initialized: 100 Hz periodic timer (IRQ 0)");
    klog_ok!("PS/2 Keyboard Driver Active: IRQ 1 unmasked");

    // Enable CPU hardware interrupts (sti)
    // SAFETY: IDT is fully populated and PIC/PIT are configured.
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
    klog_ok!("Hardware Interrupts Enabled (STI active)");

    // Wait for at least 10 timer ticks to prove hardware interrupts are delivering cleanly
    let start_ticks = pit::get_ticks();
    while pit::get_ticks() < start_ticks + 10 {
        core::hint::spin_loop();
    }
    let elapsed = pit::get_ticks() - start_ticks;
    let total_irqs = interrupts::get_total_interrupts();
    klog_ok!("Timer Verification: {} ticks received (Total IRQs: {})", elapsed, total_irqs);

    // 9. Phase 5: Physical Memory Management (E820 parsing & Frame Allocator)
    if let Some(info) = mb_info {
        // SAFETY: mmap_addr and mmap_length point to bootloader-prepared memory map.
        let mmap_summary = unsafe { mmap::parse_mmap(info.mmap_addr as u64, info.mmap_length) };
        klog_ok!(
            "E820 Memory Map: {} regions, {} MiB usable RAM (Highest: {:#010x})",
            mmap_summary.region_count,
            mmap_summary.total_usable_bytes / (1024 * 1024),
            mmap_summary.highest_usable_address
        );

        let mut allocator = FRAME_ALLOCATOR.lock();
        allocator.init(&mmap_summary, layout.kernel_start as u64, layout.kernel_end as u64);
        klog_ok!(
            "Frame Allocator Initialized: {} total frames, {} free frames ({} MiB free)",
            allocator.total_frames(),
            allocator.free_frames(),
            (allocator.free_frames() * PAGE_SIZE) / (1024 * 1024)
        );
        drop(allocator);

        // 10. Phase 6: Paging + Virtual Memory (4-Level Page Tables & Dynamic Mapping)
        paging::init(cr3);
        klog_ok!("Paging Initialized: Active PML4 at physical {:#010x}", cr3);

        // 11. Phase 7: Kernel Heap + Rust alloc support
        heap::init().expect("Kernel heap initialization failed");
        let heap_stats = ALLOCATOR.0.lock();
        klog_ok!(
            "Kernel Heap Initialized: [{:#018x} .. {:#018x}] ({} KiB total)",
            heap::HEAP_START,
            heap::HEAP_START + (heap::HEAP_INITIAL_SIZE as u64),
            heap_stats.total_bytes() / 1024
        );
        drop(heap_stats);

        // Milestone C Test 1: Box allocation
        let heap_box = Box::new(0xCAFE_BABE_DEAD_BEEFu64);
        if *heap_box != 0xCAFE_BABE_DEAD_BEEFu64 {
            panic!("Heap Box value mismatch: {:#x}", *heap_box);
        }
        klog_ok!("Heap Box Test Passed: value = {:#018x}", *heap_box);
        drop(heap_box);

        // Milestone C Test 2: Dynamic Vec allocation and growth
        let mut vec = Vec::new();
        for i in 0..500 {
            vec.push(i * 2);
        }
        if vec.len() != 500 || vec[250] != 500 {
            panic!("Heap Vec data integrity failure");
        }
        klog_ok!("Heap Vec Test Passed: 500 elements pushed, vec[250] = {}", vec[250]);
        drop(vec);

        // Milestone C Test 3: String formatting and dynamic concatenation
        let mut msg = String::from("Integrate OS");
        msg.push_str(" - Kernel Heap Active");
        if msg.len() != 33 {
            panic!("Heap String length mismatch: {}", msg.len());
        }
        klog_ok!("Heap String Test Passed: '{}'", msg.as_str());
        drop(msg);

        // Milestone C Test 4: Repeated allocation and deallocation stability test
        for cycle in 0..100 {
            let mut temp_vec = vec![cycle; 64];
            temp_vec[0] += 1;
        }
        let final_stats = ALLOCATOR.0.lock();
        klog_ok!(
            "Heap Stability Cycles Passed: 100 alloc/free iterations (Allocated: {} bytes, Free: {} bytes)",
            final_stats.allocated_bytes(),
            final_stats.free_bytes()
        );
        drop(final_stats);
        klog_ok!("MILESTONE C VERIFIED: Rust dynamic allocation fully functional");

        // 12. Phase 8: Driver Architecture & Device Registry
        manager::init().expect("Device manager initialization failed");
        let dev_mgr = DEVICE_MANAGER.lock();
        klog_ok!(
            "Device Registry Active: {} total drivers registered",
            dev_mgr.count()
        );
        klog_info!(
            "  Hardware Status: {} SUPPORTED, {} FOUNDATION, {} PARTIAL, {} UNSUPPORTED",
            dev_mgr.count_by_status(HardwareSupportStatus::Supported),
            dev_mgr.count_by_status(HardwareSupportStatus::Foundation),
            dev_mgr.count_by_status(HardwareSupportStatus::Partial),
            dev_mgr.count_by_status(HardwareSupportStatus::Unsupported)
        );

        for i in 0..dev_mgr.count() {
            if let Some(d) = dev_mgr.get(i) {
                klog_info!(
                    "  Dev {:02}: [{:<10}] {:<32} ({})",
                    i,
                    d.category().as_str(),
                    d.name(),
                    d.status().as_str()
                );
            }
        }
        drop(dev_mgr);
        klog_ok!("Driver Framework Initialized: Central registry operational");

        // 13. Phase 9: Framebuffer + Graphics Engine & Milestone G Verification
        crate::graphics::with_canvas(|canvas| {
            crate::graphics::demo::run_milestone_g_tests(canvas);
        });

        // 14. Phase 10: User Mode (Ring 3 Privilege Separation & System Calls)
        user::run_user_mode_tests();

        // 15. Phase 11: Processes (PCB, PID Allocation, Context Switch & Lifecycle Management)
        process::init().expect("Process manager initialization failed");
        process::tests::run_process_tests();

        // 16. Phase 12: System Calls & Milestone D (ABI Dispatch, Pointer Security, Ring 3 int 0x80)
        syscall::init();
        syscall::tests::run_syscall_tests();

        // 17. Phase 13: Scheduler & Multitasking (Priority Run Queues, Preemption, Sleep, Cooperative Yield)
        scheduler::init();
        scheduler::tests::run_scheduler_tests();

        // 18. Phase 14: Storage & Milestone E (IDE / ATA Block Storage Driver)
        storage::init();
        storage::tests::run_storage_tests();

        // 19. Phase 15: Filesystem + VFS (Virtual Filesystem, Inodes, Directory Trees)
        fs::init();
        fs::tests::run_fs_tests();

        // 20. Phase 16: Input Subsystem (Keyboard, PS/2 Mouse, Virtual Cursor, Event Queue)
        input::init();
        input::tests::run_input_tests();

        // 21. Phase 17: Window Manager (Z-Order, Decorations, Drag, Hit-Test)
        wm::init();
        wm::tests::run_wm_tests();

        // 22. Phase 18: Compositor (Damage Tracking, Drop Shadows, Taskbar, Multi-Layer Composition)
        compositor::init();
        compositor::tests::run_compositor_tests();

        // 23. Phase 19: GUI Toolkit (Widgets, Layout, Events, Rendering)
        gui::init();
        gui::tests::run_gui_tests();

        // 24. Phase 20: Integrate OS Desktop (Icons, Start Menu, Watermark, App Launch)
        desktop::init();
        desktop::tests::run_desktop_tests();

        // 25. Phase 21: Application Framework (Lifecycle, AppEnv, Event Bus, Clipboard, Runtime)
        framework::init();
        framework::tests::run_framework_tests();

        // 26. Phase 22: Desktop Applications (Terminal, FileManager, SysMon, Editor, Settings)
        apps::init();
        apps::tests::run_apps_tests();

        // 27. Phase 24: PCI / PCIe (Bus Enumeration, Configuration Space, BAR Decoding, Capabilities)
        pci::init();
        pci::tests::run_pci_tests();

        // 28. Phase 23: Networking (Ethernet, ARP, IPv4, ICMP, UDP, TCP, Sockets, Loopback & E1000)
        net::init();
        net::tests::run_net_tests();

        // 29. Phase 25: USB (Controller Detection, Bus Descriptors, HID Keyboard/Mouse, Mass Storage BOT)
        usb::init();
        usb::tests::run_usb_tests();

        // 30. Phase 26: GPU Foundation (PCI Display Adapter Identification, Graphics Device Abstraction, Mode Setting)
        gpu::init();
        gpu::tests::run_gpu_tests();

        // 31. Phase 27: Audio (PCI Audio Controller Detection, PCM Buffer Abstraction, Synthesizer & Playback API)
        audio::init();
        audio::tests::run_audio_tests();

        // 32. Phase 28: RTC + Time (CMOS RTC Driver, Gregorian Calendar, Wall Clock, Monotonic Uptime, Timers)
        time::init();
        time::tests::run_time_tests();

        // 33. Phase 29: ACPI + Power Management (RSDP/RSDT/XSDT Discovery, FADT, MADT, Power Transitions)
        acpi::init();
        acpi::tests::run_acpi_tests();

        // 34. Phase 30: Security + Isolation (User Pointer Validation, Capabilities, Sandboxing, Resource Limits)
        security::init();
        security::tests::run_security_tests();

        // 35. Phase 31: Kernel Synchronization (Atomics, Spinlocks, RwLocks, Mutexes, Wait Queues)
        sync::init();
        sync::tests::run_sync_tests();

        // 36. Phase 32: Debugging + Reliability (Structured Logger, Exception Decoder, Stack Unwinding, Error Codes)
        debug::init();
        debug::tests::run_debug_tests();

        // 37. Phase 33: Full Integration (Central Registry, Cross-Subsystem Matrix, Telemetry, Regression Suite)
        integration::init();
        integration::suite::run_integration_tests();

        // 38. Phase 34: Research Incorporation (Processor Affinity, Workload Distribution, CPU-Time Telemetry)
        research::init();
        research::tests::run_research_tests();

        // 39. Phase 35: Baseline Boot-Time Profiler + Network Connectivity (LAN & WiFi)
        net::phase35_tests::run_phase35_tests();
        net::wifi_real_tests::run_all();

        // 40. Phase 36: Laptop Hardware Compatibility (Intel i7-1355U, Iris Xe, NVMe, ACPI)
        drivers::laptop::init();
        drivers::laptop_tests::run_all();

        // 41. Phase 37: In-Built Package Manager (APTITUDE alias APT for Debian)
        pkg::init();
        pkg::tests::run_all();

        // 42. Phase 38: In-Built Store (snapstore) & Boot Time Comparison Table
        apps::snapstore::init();
        apps::snapstore_tests::run_all();

        // 43. Direct SSD / Storage Mount (/mnt/ssd with --read-only hardware safety shield)
        storage::ssd::init_and_mount_ssd();

        // 44. Launch Live Interactive Desktop Environment (PC App, Terminal, Icons, Mouse & Keyboard)
        desktop::runtime::run_interactive_desktop();
    } else {
        panic!("Missing Multiboot information structure for memory initialization");
    }
}

/// Halts the logical processor indefinitely with interrupts enabled/disabled.
#[inline(always)]
pub fn halt_loop() -> ! {
    loop {
        // SAFETY: Parking processor with hlt.
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Bare-metal kernel panic handler with dual VGA + Serial diagnostic reporting.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(Color::White, Color::Red);
    kprintln!("\n========================================");
    kprintln!("         INTEGRATE OS KERNEL PANIC      ");
    kprintln!("========================================");
    vga::set_color(Color::LightRed, Color::Black);
    if let Some(loc) = info.location() {
        kprintln!("Location : {}:{}:{}", loc.file(), loc.line(), loc.column());
    }
    kprintln!("Message  : {}", info.message());

    loop {
        // SAFETY: Halt with interrupts disabled on fatal panic.
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
