# INTEGRATE OS — COMPLETE OS ARCHITECTURE AUDIT (PHASE 0)

**Repository:** `D:\IntegrateOS`  
**Target Architecture:** `x86_64-unknown-none` (64-bit Bare-Metal Rust, `#![no_std]`, `#![no_main]`)  
**Audit Date:** 2026-11-18  

---

## 1. Current Architecture Overview

Integrate OS is a monolithic 64-bit bare-metal kernel written in pure Rust (`kernel/src/`).

### Boot & Execution Flow
1. **BIOS / El Torito / Multiboot1**:
   - `scripts/iso.ps1` builds a hybrid ISO-9660 + El Torito bootable image (`build/IntegrateOS.iso`).
   - Stage 2 bootloader loads the ELF64 kernel (`integrate-kernel`) and jumps to `_start` in 32-bit protected mode with `EAX = 0x2BADB002` (Multiboot1 magic) and `EBX = multiboot_info_addr`.
2. **Bootstrap Assembly (`kernel/src/boot.rs`)**:
   - Sets up a 64 KiB bootstrap stack (`KERNEL_BOOT_STACK`).
   - Identity-maps the first 4 GiB of physical address space (`0x0000_0000 .. 0xFFFF_FFFF`) using 2 MiB huge pages (`boot_pml4`, `boot_pdpt`, `boot_pd[0..4]`).
   - Enables PAE (`CR4.PAE`), Long Mode (`IA32_EFER.LME`), Paging (`CR0.PG`), and Write Protect (`CR0.WP`).
   - Loads a 64-bit GDT and performs a far return (`retf`) into `long_mode_start`, which calls `kernel_main(multiboot_magic, multiboot_info_addr)`.
3. **Kernel Initialization (`kernel/src/main.rs`)**:
   - Initializes Serial COM1 (`0x3F8`), VGA text mode (`0xB8000`), GDT/TSS, IDT (256 vectors + IST1 for `#DF`), 8259 PIC (vectors `32..47`), 8254 PIT (100 Hz), and PS/2 keyboard (`IRQ 1`).
   - Parses the Multiboot E820 memory map (`memory/mmap.rs`), initializes the physical bitmap frame allocator (`memory/frame_allocator.rs`), 4-level page manager (`memory/paging.rs`), and 32 MiB kernel heap at `0xFFFF_9000_0000_0000` (`memory/heap.rs`).
   - Initializes all kernel subsystems: `drivers`, `graphics`, `user`, `process`, `syscall`, `scheduler`, `storage`, `fs`, `input`, `wm`, `compositor`, `gui`, `desktop`, `framework`, `apps`, `net`, `pci`, `usb`, `gpu`, `audio`, `time`, `acpi`, `security`, `sync`, `debug`, `integration`, `research`, `pkg`.

---

## 2. Exact Root Cause Analysis of Reported Issues

### Issue A: Why Keyboard and Mouse Did Not Respond in VirtualBox
Tracing the complete path `Hardware -> 8042 Controller -> PIC IRQ 1/12 -> Driver -> InputEventQueue -> WindowManager -> Application` revealed **three concrete root causes**:

1. **`kernel_main()` Terminated in `halt_loop()` Instead of Running the Interactive Desktop Loop**:
   - At the end of `kernel_main()` (`kernel/src/main.rs:406`), after running all boot self-tests, the kernel called `halt_loop()` (`loop { hlt }`).
   - Although IRQ 1 (keyboard) and IRQ 12 (mouse) pushed events into `input::EVENT_QUEUE`, **nothing in `kernel_main()` ever dequeued `input::pop_event()` after boot**, nothing updated the `WindowManager` or `DesktopShell`, and nothing re-rendered the screen or mouse cursor!
2. **8042 PS/2 Dual-Channel Buffer Contention (`0x60` / `0x64`)**:
   - In `kernel/src/input/mouse.rs:326`, `handle_mouse_interrupt()` checked `if (status & 0x20) == 0 { return; }` without reading port `0x60`. If a keyboard byte arrived in the single shared 8042 output buffer while IRQ 12 was asserted, returning without reading `0x60` left `OBF=1` (Output Buffer Full) permanently, wedging the 8042 controller so neither IRQ 1 nor IRQ 12 could ever fire again.
   - Conversely, in `kernel/src/arch/x86_64/keyboard.rs:79`, `handle_keyboard_interrupt()` did not check bit 5 (`status & 0x20`), so it could consume an auxiliary mouse packet byte and misinterpret it as a keyboard scancode, desynchronizing the 3-byte mouse packet state machine.
3. **VirtualBox Mouse Integration Notice**:
   - VirtualBox displays *"The Virtual Machine reports that the guest OS does not support mouse pointer integration"* whenever the guest uses relative PS/2 mouse packets instead of absolute USB HID Tablet / VMMDev coordinates. Once the user clicks **Capture**, VirtualBox sends standard PS/2 relative motion and button packets on IRQ 12 (`0x60`/`0x64`). Fixing the 8042 drain and running the live compositor loop makes the captured mouse cursor move smoothly across the entire screen.

### Issue B: Why the Display Was Stuck on the Milestone G Screen or Black Screen
1. **VRAM Wiped by Phase 26 GPU Mode Switch Test**:
   - Phase 9 (`graphics::demo::run_milestone_g_tests`) drew the static Milestone G showcase to physical VRAM (`0xE000_0000`).
   - Later in boot, Phase 26 (`gpu::tests::run_gpu_tests`) tested mode switching (`1024x768 -> 800x600 -> 1024x768`) via `bga::set_video_mode()`, which toggled `VBE_DISPI_INDEX_ENABLE` without `VBE_DISPI_NOCLEARMEM` and without re-presenting the `DesktopShell` canvas afterward, leaving VRAM completely black (or frozen on Milestone G in earlier builds).
2. **Static Demo vs. Live Compositor**:
   - The buttons `[Terminal] [Files] [Editor] [System] [Settings]` drawn in `graphics/demo.rs` were static rectangles painted during Phase 9, not connected to `DesktopShell`, `WindowManager`, or `AppManager`.

---

## 3. Subsystem-by-Subsystem Audit & Classification

| Subsystem | Module Path | Status | Assessment |
| :--- | :--- | :--- | :--- |
| **Bootloader & Entry** | `kernel/src/boot.rs`, `scripts/iso.ps1` | `COMPLETE` | Multiboot1 + El Torito Stage 2, 4 GiB identity map, 64-bit Long Mode entry. |
| **CPU, GDT, TSS, IDT** | `kernel/src/arch/x86_64/` | `COMPLETE` | Ring 0/Ring 3 segments, TSS with `RSP0`/`IST1`, 256-entry IDT, exception recovery. |
| **Interrupts (PIC/PIT)** | `kernel/src/arch/x86_64/pic.rs`, `pit.rs` | `COMPLETE` | 8259 PIC remapped (`32..47`), 8254 PIT at 100 Hz. |
| **Physical Memory** | `kernel/src/memory/mmap.rs`, `frame_allocator.rs` | `COMPLETE` | E820 parser + bitmap physical frame allocator. |
| **Paging & Heap** | `kernel/src/memory/paging.rs`, `heap.rs` | `COMPLETE` | 4-level page tables + 32 MiB linked-list kernel heap (`GlobalAlloc`). |
| **Processes & Scheduler** | `kernel/src/process/`, `scheduler/` | `COMPLETE` | PCB, PID allocator, priority run queues, CPU affinity masks, preemptive ticks. |
| **System Calls** | `kernel/src/syscall/`, `user/` | `COMPLETE` | `int 0x80` & `SYSCALL` MSR dispatcher, pointer validation. |
| **PCI / PCIe Bus** | `kernel/src/pci/` | `COMPLETE` | Recursive bus/slot/function scanner, BAR sizing, capabilities, class decoding. |
| **Storage (IDE/NVMe/AHCI)** | `kernel/src/storage/` | `PARTIAL` | ATA PIO driver works on `/dev/hda`; needs PCI NVMe & AHCI discovery + read-only SSD safety mount `/mnt/ssd`. |
| **Filesystem & VFS** | `kernel/src/fs/` | `PARTIAL` | In-memory VFS (`RamFs`) + `/dev` nodes work; needs partition mount table (`/mnt/ssd`), `lsblk`, `df`, `mount`. |
| **Input (PS/2 & USB HID)** | `kernel/src/input/`, `usb/` | `BROKEN` -> `FIXING` | PS/2 keyboard & mouse drivers exist but need unified 8042 drain and live runtime event loop dispatch. |
| **Graphics & Framebuffer** | `kernel/src/graphics/`, `gpu/` | `PARTIAL` | BGA LFB (`0xE000_0000`) works; needs `VBE_DISPI_NOCLEARMEM`, dynamic resolution manager, and live desktop presentation. |
| **Window Manager & GUI** | `kernel/src/wm/`, `compositor/`, `gui/` | `PARTIAL` | Z-order, dragging, buttons, text inputs work in unit tests; needs live interactive loop binding in `kernel_main`. |
| **Desktop & Icons** | `kernel/src/desktop/` | `PARTIAL` | Wallpaper, icons, Start menu exist; needs **"PC" App icon**, single/double-click launch into `AppManager`, and live cursor redraw. |
| **Command-Line Shell** | `kernel/src/apps/terminal.rs` | `PARTIAL` | Basic commands exist; needs full POSIX-style shell (`hwinfo`, `lspci`, `lsblk`, `mount`, `df`, `ifconfig`, `ip`, `ping`, `wifi`, `pkg`, `apt`, `>`, `\|`, history, tab completion). |
| **Package Manager (APT/PKG)** | `kernel/src/pkg/` | `PARTIAL` | `apt` & `aptitude` work; needs `pkg` command family, SHA-256/FNV package checksum verification, rollback, and VFS `/usr/bin` binary installation. |
| **Networking & WiFi** | `kernel/src/net/` | `PARTIAL` | Ethernet/IPv4/ICMP/UDP/TCP/WiFi state machine exist; needs live hardware NIC PCI ID correlation, IP config, and interactive WiFi connect in Shell & **PC** App. |
| **"PC" App (SSD & Hardware)** | `kernel/src/apps/pc.rs` | `MISSING` -> `ADDING` | Dedicated **"PC"** desktop application showing mounted SSD (`/mnt/ssd` + `/dev/hda`), real PCI/Hardware device IDs, laptop IP address, and nearby WiFi connection UI. |

---

## 4. Recommended Dependency & Implementation Order (Phases 1–20)

1. **Phase 1 & 2**: Unified Hardware Discovery (`hwinfo`) & Live PCI/PCIe Bus Enumeration (`lspci`) reading actual hardware registers and PCI configuration space.
2. **Phase 3 & 4**: Direct SSD / NVMe / AHCI / ATA Controller Discovery with strict Read-Only Hardware Safety Lock (`--read-only` by default), Partition Discovery, and mounting into `/mnt/ssd` (`lsblk`, `mount`, `df`, `mounts`).
3. **Phase 5, 6 & 7**: Full Command-Line Shell (`$` prompt, 35+ commands, redirection `>`, pipes `|`, history, tab completion), Verified APT/PKG Package Manager (checksum verification + VFS binary installation), and Networking/WiFi Diagnostics (`ifconfig`, `ip`, `ping`, `route`, `dns`, `netstat`, `wifi scan`, `wifi connect`).
4. **Phase 8, 9 & 10**: Unified 8042 PS/2 Keyboard & Mouse Interrupt/Poll Multiplexer + USB HID Input Bridge + Dynamic VirtualBox/BGA Display Resolution Engine (`1024x768` / `1280x800` with `VBE_DISPI_NOCLEARMEM`).
5. **Phase 11, 12, 13, 14, 15, 16**: Live Interactive Desktop Runtime Loop (`kernel/src/desktop/runtime.rs`), **"PC" App (`kernel/src/apps/pc.rs`)** with mounted SSD explorer, hardware IDs, IP & WiFi connect button, plus interactive **Terminal**, **Files**, **System Monitor**, **Package Manager**, and **Settings** apps.
6. **Phase 17, 18, 19, 20**: Syscall/Security hardening, `docs/TEST_PLAN.md`, `docs/REAL_HARDWARE_COMPATIBILITY.md`, Release ISO rebuild, and live VirtualBox GUI & Serial verification.
