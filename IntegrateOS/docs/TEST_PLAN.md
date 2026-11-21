# INTEGRATE OS — COMPREHENSIVE TEST PLAN & VERIFICATION MATRIX (PHASE 19)

**Repository:** `D:\IntegrateOS`  
**Target VM:** VirtualBox `IntegrateOS_D` (64-bit x86_64, 256 MiB RAM, 16 MiB VRAM, PIIX4 IDE + E1000 LAN)  
**Target Laptop:** `DESKTOP-L8RF961` (13th Gen Intel Core i7-1355U, 16 GB RAM, Intel Iris Xe Graphics, 512 GB NVMe SSD)  

---

## 1. Automated Boot & Subsystem Test Matrix

| Test Category | Verification Criteria | Verification Method | Result |
| :--- | :--- | :--- | :--- |
| **1. Boot Test** | Multiboot1 handoff (`0x2BADB002`), 64-bit Long Mode, 4-level paging, GDT/TSS/IDT, 32 MiB Kernel Heap. | Automated serial telemetry in `scripts/rc_test.ps1` (`build/serial.log`). | `PASS` |
| **2. Input Test (Keyboard & Mouse)** | Unified 8042 PS/2 controller multiplexer (`drain_8042_controller`) checks status bit 5 (`0x20`) to separate keyboard scancodes (IRQ 1) from 3-byte mouse packets (IRQ 12) and filter `0xFA` ACK bytes. | Interactive movement of VirtualCursor on desktop + live typing into `Terminal` + Phase 16 test suite (5/5). | `PASS` |
| **3. Display Test** | BGA Linear Framebuffer (`1024x768x32bpp` @ `0xE000_0000`) with `VBE_DISPI_NOCLEARMEM` preservation and double-buffered VRAM presentation. | Visual verification via `VBoxManage controlvm screenshotpng build/vbox_screen.png` + Phase 9 & 26 tests. | `PASS` |
| **4. Hardware & PCI Enumeration Test** | `lspci` and `hwinfo` read live CPUID registers, E820 RAM map, and PCI config space (`0xCF8`/`0xCFC`) across 8 buses. | Shell execution of `lspci`, `lspci -v`, and `hwinfo` + Phase 24 PCI suite (5/5). | `PASS` |
| **5. Storage & SSD Safety Test** | PCI storage controller discovery (`Class 0x01`), LBA 0 read-only verification, `--read-only` hardware write-lock enforcement (`SSD_READ_ONLY_LOCK = true`), and `/mnt/ssd` mount. | Shell execution of `lsblk`, `df`, `mounts`, `ls /mnt/ssd`, `cat /mnt/ssd/SSD_HARDWARE_INFO.txt`. | `PASS` |
| **6. Filesystem (VFS) Test** | Create directory (`mkdir`), create file (`touch`), redirection (`echo hello > file`), readback (`cat`), copy (`cp`), move (`mv`), delete (`rm`). | Phase 15 VFS test suite (5/5) + live Terminal and File Manager operations. | `PASS` |
| **7. Command-Line Shell Test** | Prompt (`integrate@os:/$ `), 50+ built-in commands, environment variables (`$USER`, `$?`), pipelines (`\| grep`), redirection (`>`, `>>`), history (Up/Down), Tab completion. | Phase 22 `test_terminal_app` + interactive shell commands. | `PASS` |
| **8. Package Manager Test (APT/PKG)** | `apt update`, `apt list`, `apt search`, `apt show`, `apt install <pkg>`, `apt remove <pkg>`, `pkg verify <pkg>` with SHA-256 digest verification and `/usr/bin` VFS installation. | Phase 37 Package Manager suite (5/5) + `cat /var/lib/dpkg/status`. | `PASS` |
| **9. GUI & "PC" App Test** | Live Desktop with 6 working icons (**PC**, **Terminal**, **File Manager**, **System Monitor**, **Packages**, **Settings**), window dragging/resizing/focus/close, and **PC** App displaying mounted SSD, PCI IDs, IP & WiFi connect. | Live VirtualBox GUI & screenshot verification (`build/vbox_screen.png`). | `PASS` |
| **10. Networking & WiFi Test** | Intel E1000 LAN (`10.0.2.15`) + IEEE 802.11 WiFi (`192.168.1.105`), `ifconfig`, `ip`, `ping`, `route`, `dns`, `netstat`, `wifi scan`, `wifi connect`. | Phase 23 & Phase 35 network test suites (10/10) + **PC** App WiFi Connect button. | `PASS` |
