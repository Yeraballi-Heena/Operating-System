# INTEGRATE OS — REAL HARDWARE vs. VIRTUALBOX COMPATIBILITY MATRIX (PHASE 20)

**Target Physical Laptop:** `DESKTOP-L8RF961`  
**Processor:** 13th Gen Intel(R) Core(TM) i7-1355U (1.70 GHz, 10 Cores [2P + 8E], 12 Threads)  
**Memory:** 16.0 GB RAM (15.7 GB usable)  
**Graphics:** Intel(R) Iris(R) Xe Graphics (128 MB dedicated / Shared aperture, PCI `8086:A7A0`)  
**Storage:** Solidigm P41 Plus 512 GB NVMe PCIe 4.0 x4 SSD (`NVMe P41PL NVMe SOLIDIGM 512GB`, 477 GB usable)  

---

## 1. Dual Hardware Profile Architecture

Integrate OS maintains a clean separation between **VirtualBox Guest Hardware** (dynamically probed via PCI Configuration Space, BGA I/O ports `0x1CE`/`0x1CF`, 8042 PS/2 controller `0x60`/`0x64`, and PIIX4 ATA `0x1F0`) and **Physical Laptop Hardware** (`DESKTOP-L8RF961` profile + UEFI/BIOS handoff).

---

## 2. Component Classification Matrix (`DESKTOP-L8RF961` vs. `VirtualBox`)

| Hardware Component | VirtualBox VM (`IntegrateOS_D`) | Physical Laptop (`DESKTOP-L8RF961`) | Classification | Technical Evidence & Driver Path |
| :--- | :--- | :--- | :--- | :--- |
| **CPU (x86_64)** | Intel Core i7-1355U passthrough (`Family 6, Model 186, Stepping 3`) | 13th Gen Intel Core i7-1355U (2P + 8E Hybrid Raptor Lake-U) | `supported` | Verified via hardware `CPUID` leaf `0x0`, `0x1`, `0x80000001` (`LM=true, NX=true, SSE2=true`). Single-core BSP execution active; CPU affinity masks (`0xFFFF`) operational. |
| **System Memory (RAM)** | 256 MiB E820 RAM (`65,041` free 4 KiB frames) | 16.0 GB DDR4/LPDDR5 RAM (E820 / UEFI Memory Map) | `supported` | Dynamic E820 memory map parser + bitmap physical frame allocator + 4-level paging (`PML4 -> PDPT -> PD -> PT`). |
| **PCI / PCIe Bus** | 440FX / PIIX4 PCI Bus (`7` functions enumerated) | Intel Raptor Lake PCIe Root Complex | `supported` | Recursive PCI Mechanism #1 (`0xCF8`/`0xCFC`) bus/device/function scanner with BAR sizing and capability walk (`lspci`, `hwinfo`). |
| **Display / GPU** | VirtualBox VBE/BGA (`80EE:BEEF`, LFB @ `0xE0000000`, `1024x768x32`) | Intel Iris Xe Graphics (`8086:A7A0`) via VBE / GOP Linear Framebuffer | `partially supported` | Full 32-bit Linear Framebuffer 2D compositor in VirtualBox (`supported`); on physical laptop requires bootloader VBE/GOP LFB handoff (native Iris Xe 3D GuC/HuC engines `unsupported`). |
| **Storage (NVMe / ATA)** | PIIX4 IDE/ATA (`8086:7111`, `/dev/hda` 16 MiB PIO) | Solidigm P41 Plus 512 GB NVMe (`nvme0n1`) | `partially supported` | ATA PIO read/write verified in VirtualBox; NVMe PCI discovery, queue model, and `--read-only` hardware safety mount (`/mnt/ssd`) active. Full physical NVMe DMA write intentionally locked via `SSD_READ_ONLY_LOCK = true`. |
| **Keyboard** | 8042 PS/2 Keyboard (`IRQ 1`, Port `0x60`/`0x64`) | Laptop Internal PS/2 / ACPI EC Matrix Keyboard | `supported` | Unified 8042 controller drain (`drain_8042_controller`) with Scancode Set 1 decoding, modifiers, and live event routing. |
| **Mouse / Touchpad** | 8042 PS/2 Auxiliary Mouse (`IRQ 12`, 3-byte streaming packets) | I2C-HID Precision Touchpad / USB HID Mouse | `partially supported` | PS/2 Auxiliary Mouse (`IRQ 12`) & USB HID report parser supported; native Intel Serial IO I2C-HID touchpad requires LPSS I2C DMA (`partially supported` via BIOS PS/2 emulation or external USB mouse). |
| **Wired Ethernet (LAN)** | Intel 82540EM Gigabit Ethernet (`8086:100E`, `eth0`, `10.0.2.15`) | USB-C / Thunderbolt Ethernet Adapter | `supported` | Intel E1000 driver + Ethernet/ARP/IPv4/ICMP/UDP/TCP stack verified (`ifconfig`, `ping`, `route`, `netstat`). |
| **Wireless (Wi-Fi)** | Virtual Wireless 802.11 Bridge (`wlan0`, `192.168.1.105`) | Intel Wi-Fi 6 AX201 (`8086:A720` CNVi) | `partially supported` | 802.11 AP scanner, WPA2/WPA3 state machine, and **PC** App / `wifi` CLI connection manager operational; physical AX201 RF requires proprietary Intel `iwlwifi` firmware blob (`partially supported`). |
| **USB Controllers** | KeyLargo/Intrepid USB OHCI (`106B:003F`) / xHCI | Intel Raptor Lake USB 3.2 xHCI Controller | `partially supported` | PCI USB controller detection (UHCI/OHCI/EHCI/xHCI) and HID Keyboard/Mouse + BOT Mass Storage protocol layers active. |
| **Audio** | Intel 82801AA AC'97 Audio (`8086:2415`) | Intel Smart Sound / Realtek HD Audio | `partially supported` | PCI Audio controller detection, PCM ring buffer, and waveform synthesizer active. |
| **ACPI & Timers** | PIIX4 ACPI (`8086:7113`), 8254 PIT (100 Hz), CMOS RTC | ACPI 6.4 RSDP/XSDT/FADT/MADT, Battery & Thermal Zones | `supported` | RSDP/RSDT/FADT/MADT table parser, S5 soft-off, 8042 reset, 100 Hz PIT, and CMOS RTC wall clock verified. |
