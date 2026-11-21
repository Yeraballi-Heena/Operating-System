# Integrate OS — System Architecture Specification

## 1. Engineering Principles

1. **Pure Bare-Metal Rust**: Compiled for `x86_64-unknown-none` using `#![no_std]` and `#![no_main]`. No host `std` library or OS emulation is used.
2. **Strict `unsafe` Isolation**: `#![deny(unsafe_op_in_unsafe_fn)]` is enforced across the kernel crate. Every `unsafe` block is isolated within low-level hardware/architecture modules (`src/arch/x86_64/`, port I/O, page table manipulation) and documented with a `// SAFETY:` justification.
3. **Separation of Concerns**:
   - `src/arch/x86_64/`: CPU registers, GDT, TSS, IDT, PIC/APIC, paging structures, context switch assembly.
   - `src/memory/`: Physical memory map parsing, physical frame allocator, virtual memory mapper, kernel heap (`alloc`).
   - `src/drivers/`: Trait-based driver framework (`DisplayDriver`, `InputDriver`, `BlockDevice`, `NetworkDevice`, etc.).
   - `src/proc/` & `src/syscall/`: Process control blocks, user-mode ring 3 isolation, preemptive scheduler, validated syscall dispatcher.
   - `src/fs/`: Block storage, RAM/disk filesystem, Virtual File System (VFS).
   - `src/graphics/`, `src/wm/`, `src/compositor/`, `src/gui/`, `src/desktop/`: Double-buffered framebuffer graphics, window management, damage-tracked compositor, widget toolkit, and desktop environment.

---

## 2. Physical & Virtual Memory Layout (Initial Foundation)

| Physical Address Range | Purpose |
| :--- | :--- |
| `0x0000_0000 .. 0x0000_0500` | Real-Mode IVT & BIOS Data Area (Reserved) |
| `0x0000_8000 .. 0x0000_A000` | Multiboot Information Structure & BIOS E820 Memory Map |
| `0x000A_0000 .. 0x000F_FFFF` | Legacy VGA Video Memory (`0xB8000`) & BIOS ROM |
| `0x0010_0000 .. __kernel_end` | Integrate OS Kernel Image (`.multiboot`, `.text`, `.rodata`, `.data`, `.bss`) |
| `__kernel_end .. Top of RAM` | Managed by Physical Frame Allocator (Phase 5) & Kernel Heap (Phase 7) |

---

## 3. Master 37-Phase Development Plan

- **Phase 0**: Development Foundation (Toolchain, Cargo, Linker, GRUB/ISO pipeline, Test scripts)
- **Phase 1**: GRUB + Rust Kernel Boot (32→64-bit Long Mode transition, stack, serial/VGA output, Milestone A VirtualBox test)
- **Phase 2**: `x86_64` CPU Architecture (GDT, kernel/user segments, TSS, privilege levels)
- **Phase 3**: IDT + CPU Exceptions (Divide error, invalid opcode, GPF, page fault, double fault, breakpoint)
- **Phase 4**: Hardware Interrupts (8259 PIC / IRQ routing, PIT timer, PS/2 keyboard, Milestone B VirtualBox test)
- **Phase 5**: Physical Memory Management (Multiboot E820 memory map, physical frame allocator)
- **Phase 6**: Paging + Virtual Memory (4-level page tables, map/unmap/translate, access permissions)
- **Phase 7**: Kernel Heap + Rust `alloc` (Kernel heap allocator, `Box`/`Vec`/`String`, Milestone C VirtualBox test)
- **Phase 8**: Driver Architecture (Unified device & driver registry and trait abstractions)
- **Phase 9**: Framebuffer + Graphics (Linear framebuffer, primitives, clipping, bitmap fonts, double buffering, Milestone G)
- **Phase 10**: User Mode (Ring 3 privilege transition, user stack & page table isolation)
- **Phase 11**: Processes (PCB, PID, address spaces, context representation, lifecycle)
- **Phase 12**: System Calls (Controlled syscall ABI, pointer validation, Milestone D VirtualBox test)
- **Phase 13**: Scheduler + Multitasking (Timer-driven preemptive round-robin/priority scheduling, Milestone E)
- **Phase 14**: Storage (Block device abstraction, ATA PIO / RAM disk driver, partitions)
- **Phase 15**: Filesystem + VFS (Inode filesystem, file/directory operations, VFS layer, Milestone F)
- **Phase 16**: Input Subsystem (PS/2 keyboard & mouse driver, unified input event queue)
- **Phase 17**: Window Manager (Windows, Z-order, focus, move/resize/minimize/maximize/close, hit testing)
- **Phase 18**: Compositor (Window backing surfaces, clipping, cursor plane, double-buffered composition, Milestone H)
- **Phase 19**: GUI Toolkit (Window, Panel, Label, Button, TextBox, TextArea, List, ScrollBar, Checkbox, Dialog, Menu, StatusBar)
- **Phase 20**: Integrate OS Desktop (Wallpaper, desktop icons, Top Bar, Dock, system menus & power dialogs)
- **Phase 21**: Application Framework (User-space/isolated application lifecycle, event loop, crash containment)
- **Phase 22**: Desktop Applications (Terminal, File Manager, Text Editor, Settings, Calculator, System Monitor, Milestone I)
- **Phase 23**: Networking (Ethernet, ARP, IPv4, ICMP, UDP, TCP, Sockets, DNS, DHCP, HTTP client)
- **Phase 24**: PCI / PCIe (PCI bus enumeration, configuration space, BAR parsing, device identification)
- **Phase 25**: USB (Controller detection, bus/descriptor foundation, HID keyboard/mouse/storage interfaces)
- **Phase 26**: GPU Foundation (PCI display adapter identification, graphics device abstraction with framebuffer baseline)
- **Phase 27**: Audio (Audio controller detection, PCM buffer abstraction, playback API)
- **Phase 28**: RTC + Time (CMOS RTC driver, wall-clock date/time, monotonic uptime clock, timer APIs)
- **Phase 29**: ACPI + Power Management (RSDP/RSDT/FADT parsing, ACPI shutdown, reset, power/battery status)
- **Phase 30**: Security + Isolation (User pointer validation, capability/permission enforcement, resource limits)
- **Phase 31**: Kernel Synchronization (Atomics, interrupt-safe spinlocks, mutexes, wait queues)
- **Phase 32**: Debugging + Reliability (Structured serial logger, panic/exception diagnostics, error codes)
- **Phase 33**: Full Integration (End-to-end subsystem integration & full regression verification)
- **Phase 34**: Bootable ISO (`IntegrateOS.iso` complete packaging & verification)
- **Phase 35**: Release Candidate + VirtualBox (Milestone J full 30-point VirtualBox test matrix)
- **Phase 36**: Portable Disk Image (`IntegrateOS.img` raw bootable disk image generation)
- **Phase 37**: USB Flashing — Final Phase (Non-destructive device inspection, explicit confirmation gate, physical USB boot)
