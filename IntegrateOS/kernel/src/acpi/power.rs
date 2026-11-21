//! Integrate OS — ACPI Power Management & Hardware Reset Engine
//!
//! Provides system power transitions: ACPI mode enablement, standard S5 system shutdown,
//! ACPI hardware reset, 8042 PS/2 controller reset pulse, and triple-fault fallback.

use crate::arch::x86_64::io::{inb, inw, outb, outw};
use super::tables::Fadt;
use super::types::AcpiError;
use crate::klog_info;

/// ACPI Power and Reset Controller.
pub struct AcpiPowerManager;

impl AcpiPowerManager {
    /// Transitions the hardware from legacy BIOS mode into ACPI hardware mode.
    pub fn enable_acpi(fadt: &Fadt) -> Result<(), AcpiError> {
        if fadt.smi_cmd_port == 0 || fadt.acpi_enable_val == 0 {
            return Ok(());
        }

        // Check if ACPI is already active via SCI_EN (bit 0 of PM1a_CNT)
        if fadt.pm1a_cnt_blk != 0 {
            // SAFETY: Reading PM1a control register.
            let pm1a_cnt = unsafe { inw(fadt.pm1a_cnt_blk) };
            if (pm1a_cnt & 0x0001) != 0 {
                return Ok(()); // Already enabled
            }
        }

        klog_info!(
            "[acpi] Enabling ACPI mode: Writing 0x{:02X} to SMI command port 0x{:04X}",
            fadt.acpi_enable_val,
            fadt.smi_cmd_port
        );

        // SAFETY: Writing ACPI_ENABLE value to the SMI_CMD port.
        unsafe {
            outb(fadt.smi_cmd_port, fadt.acpi_enable_val);
        }

        // Wait for SCI_EN to be set by hardware (up to 100,000 spin loops)
        if fadt.pm1a_cnt_blk != 0 {
            let mut spins = 0;
            loop {
                // SAFETY: Reading PM1a status.
                let pm1a_cnt = unsafe { inw(fadt.pm1a_cnt_blk) };
                if (pm1a_cnt & 0x0001) != 0 {
                    break;
                }
                spins += 1;
                if spins > 100_000 {
                    return Err(AcpiError::HardwareFault);
                }
                core::hint::spin_loop();
            }
        }

        Ok(())
    }

    /// Reboots the computer using the most reliable hardware mechanism available:
    /// 1. ACPI Reset Register (FADT)
    /// 2. 8042 Keyboard Controller Reset Pulse (Port 0x64 -> 0xFE)
    /// 3. Processor Triple Fault
    pub fn reboot(fadt: Option<&Fadt>) -> ! {
        klog_info!("[acpi] Initiating system reset / reboot sequence...");

        // 1. Try ACPI Reset Register if supported
        if let Some(f) = fadt {
            if f.supports_acpi_reset() {
                let reset_addr = f.reset_reg.address;
                let reset_val = f.reset_value;
                let is_io = f.reset_reg.is_io_port();
                let is_mem = f.reset_reg.is_memory();
                klog_info!(
                    "[acpi] Triggering ACPI reset register at 0x{:08X} (val: 0x{:02X})",
                    reset_addr,
                    reset_val
                );
                // SAFETY: Performing ACPI reset write.
                unsafe {
                    if is_io {
                        outb(reset_addr as u16, reset_val);
                    } else if is_mem {
                        core::ptr::write_volatile(reset_addr as *mut u8, reset_val);
                    }
                }
                spin_delay(50);
            }
        }

        // 2. Try 8042 PS/2 Keyboard Controller Reset Pulse
        klog_info!("[acpi] Triggering 8042 Keyboard Controller reset pulse (Port 0x64 <- 0xFE)");
        // SAFETY: Direct port I/O to 8042 command register.
        unsafe {
            // Drain 8042 input buffer
            let mut spins = 0;
            while (inb(0x64) & 0x02) != 0 && spins < 100_000 {
                spins += 1;
                core::hint::spin_loop();
            }
            // Pulse CPU reset line (command 0xFE)
            outb(0x64, 0xFE);
        }
        spin_delay(50);

        // 3. Fallback: Force CPU Triple Fault by loading zero-limit IDT and firing interrupt
        klog_info!("[acpi] Triggering processor triple-fault fallback...");
        #[repr(C, packed)]
        struct NullIdtr {
            limit: u16,
            base: u64,
        }
        let null_idtr = NullIdtr { limit: 0, base: 0 };
        // SAFETY: Intentional triple fault to trigger motherboard hardware reset.
        unsafe {
            core::arch::asm!("lidt [{}]", in(reg) &null_idtr, options(readonly, nostack));
            core::arch::asm!("int 3", options(noreturn));
        }
    }

    /// Powers off / shuts down the computer (ACPI S5 Soft Off).
    pub fn power_off(fadt: Option<&Fadt>) -> ! {
        klog_info!("[acpi] Initiating ACPI S5 Soft Off (Shutdown)...");

        // 1. Try ACPI S5 sleep state via PM1 control blocks
        if let Some(f) = fadt {
            if f.pm1a_cnt_blk != 0 {
                // Typical SLP_TYPa for S5 is 0x0005, SLP_EN bit is 1 << 13 (0x2000)
                let slp_val = (5 << 10) | (1 << 13);
                // SAFETY: Writing S5 shutdown command to PM1a and PM1b control blocks.
                unsafe {
                    outw(f.pm1a_cnt_blk, slp_val as u16);
                    if f.pm1b_cnt_blk != 0 {
                        outw(f.pm1b_cnt_blk, slp_val as u16);
                    }
                }
                spin_delay(50);
            }
        }

        // 2. Hypervisor Direct Shutdown Ports
        // SAFETY: Hypervisor power-off I/O port triggers.
        unsafe {
            // QEMU ACPI power-off port
            outw(0x0604, 0x2000);
            // VirtualBox ACPI power-off port
            outw(0x4004, 0x3400);
            // Bochs / Old QEMU power-off port
            outw(0xB004, 0x2000);
        }

        // 3. Halt loop if hardware hasn't powered off yet
        loop {
            // SAFETY: Halting processor.
            unsafe {
                core::arch::asm!("cli; hlt", options(nomem, nostack));
            }
        }
    }
}

/// Brief spin-delay helper for hardware settling.
fn spin_delay(loops: usize) {
    for _ in 0..loops * 10_000 {
        core::hint::spin_loop();
    }
}
