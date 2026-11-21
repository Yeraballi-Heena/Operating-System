# Integrate OS

**Production-Grade 64-Bit Bare-Metal Operating System in Rust (`#![no_std]`, `#![no_main]`)**

---

## 1. Overview

**Integrate OS** is a 64-bit (`x86_64`) bare-metal operating system written in Rust from absolute zero. Its foundational kernel progression follows the conceptual sequence of *The Little Book About Operating System Development* (Boot → Kernel → Segmentation/GDT → Interrupts/IDT → Keyboard/Input → User Mode → Paging → Memory Allocation → User Applications → Filesystem → System Calls → Multitasking) and extends that foundation to a full 64-bit desktop operating system with graphics, window management, compositing, networking, PCI/USB/ACPI/audio subsystems, and bootable ISO/disk image generation.

---

## 2. Boot Architecture & 64-Bit GRUB Multiboot Adaptation

Following *The Little Book About Operating System Development*, **GRUB** serves as the bootloader and loads the kernel via the **Multiboot** specification:

```text
Firmware / BIOS
      ↓
GRUB / El Torito Multiboot Loader (boot/grub/grub.cfg)
      ↓
Multiboot-Compatible ELF Kernel (/boot/integrate-os.bin, magic 0x1BADB002)
      ↓
32-Bit Protected Mode Entry (_start) → 4-Level Paging + Long Mode Transition
      ↓
Integrate OS 64-Bit Rust Kernel (kernel_main)
```

### Technical Note on 64-Bit Multiboot Entry
Standard BIOS GRUB Multiboot1 (`0x1BADB002`) transfers CPU control in **32-bit Protected Mode** with paging disabled (`EAX = 0x2BADB002`, `EBX = physical address of multiboot_info_t`). Because Integrate OS is a **64-bit (`x86_64`)** kernel, the kernel ELF embeds a 32-bit bootstrap entry stub (`.text.boot`) that validates the Multiboot handoff, initializes identity-mapped 4-level page tables (PML4 → PDPT → PD), enables PAE (`CR4.PAE`), sets Long Mode Enable (`IA32_EFER.LME`), enables paging (`CR0.PG`), loads the 64-bit GDT, and performs a far jump into 64-bit Rust code.

---

## 3. Hardware Support Classification Matrix

Integrate OS explicitly classifies hardware support into four honest tiers:

| Tier | Meaning |
| :--- | :--- |
| `SUPPORTED` | Fully implemented and verified in VirtualBox / target x86_64 hardware |
| `PARTIAL` | Core baseline implemented; advanced/vendor-specific features omitted |
| `FOUNDATION` | Architectural interfaces, enumeration, and driver model implemented |
| `UNSUPPORTED` | Not supported (e.g., proprietary modern 3D GPUs, proprietary Wi-Fi chipsets) |

---

## 4. Reproducible Build, ISO, and Test Commands

### Build Bare-Metal Kernel (Debug & Release)
```powershell
# Debug build + ELF64/Multiboot verification
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1

# Release build + ELF64/Multiboot verification
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1 -Release
```

### Generate Bootable GRUB / El Torito ISO (`build\IntegrateOS.iso`)
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\iso.ps1 -Release
```

### Run Full Automated Verification Suite
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test.ps1
```

### Launch in Oracle VirtualBox
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\vbox.ps1
```
