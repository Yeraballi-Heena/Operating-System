//! Integrate OS — Central ACPI Subsystem Manager
//!
//! Orchestrates the discovery of the ACPI Root System Description Pointer (RSDP),
//! parses the description table hierarchy (RSDT, XSDT, FADT, MADT),
//! and exposes global power management (reboot, shutdown, sleep states).

use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

use super::power::AcpiPowerManager;
use super::rsdp::find_rsdp;
use super::sdt::Rsdp;
use super::tables::{AcpiTables, Fadt, Madt};
use super::types::{AcpiError, AcpiSignature};

pub struct AcpiManager {
    rsdp: Option<Rsdp>,
    tables: AcpiTables,
    fadt: Option<Fadt>,
    madt: Option<Madt>,
}

impl AcpiManager {
    pub fn new() -> Self {
        Self {
            rsdp: None,
            tables: AcpiTables::new(),
            fadt: None,
            madt: None,
        }
    }

    /// Initializes the ACPI subsystem, discovering the RSDP and parsing all tables.
    pub fn init(&mut self) -> Result<(), AcpiError> {
        let rsdp = match find_rsdp() {
            Some(r) => {
                klog_info!(
                    "[acpi] RSDP verified: OEM='{}', Rev={}, RootTable=0x{:08X}",
                    r.oem_id_str(),
                    r.revision,
                    r.root_table_address()
                );
                r
            }
            None => {
                klog_info!("[acpi] No ACPI RSDP found in EBDA/BIOS memory range");
                return Err(AcpiError::RsdpNotFound);
            }
        };

        // Parse root table pointers (RSDT / XSDT)
        self.tables.parse_from_rsdp(&rsdp)?;

        // Parse Fixed ACPI Description Table (FADT)
        if let Some(fadt_entry) = self.tables.find(AcpiSignature::FADT) {
            match Fadt::parse(fadt_entry) {
                Ok(fadt) => {
                    klog_ok!(
                        "[acpi] FADT active: PM1a_CNT=0x{:04X}, SMI_CMD=0x{:04X}, ResetSupported={}",
                        fadt.pm1a_cnt_blk,
                        fadt.smi_cmd_port,
                        fadt.supports_acpi_reset()
                    );
                    let _ = AcpiPowerManager::enable_acpi(&fadt);
                    self.fadt = Some(fadt);
                }
                Err(e) => {
                    klog_info!("[acpi] FADT parse notice: {}", e.as_str());
                }
            }
        }

        // Parse Multiple APIC Description Table (MADT)
        if let Some(madt_entry) = self.tables.find(AcpiSignature::MADT) {
            match Madt::parse(madt_entry) {
                Ok(madt) => {
                    klog_ok!(
                        "[acpi] MADT active: LAPIC=0x{:08X}, CPUs={}, IO-APICs={}, ISOs={}",
                        madt.local_apic_address,
                        madt.num_local_apics,
                        madt.num_io_apics,
                        madt.num_iso
                    );
                    self.madt = Some(madt);
                }
                Err(e) => {
                    klog_info!("[acpi] MADT parse notice: {}", e.as_str());
                }
            }
        }

        self.rsdp = Some(rsdp);
        klog_ok!("[acpi] ACPI Subsystem fully operational");

        Ok(())
    }

    /// Reboots the computer via ACPI, keyboard controller, or processor triple-fault.
    pub fn reboot(&self) -> ! {
        AcpiPowerManager::reboot(self.fadt.as_ref())
    }

    /// Powers off the computer via ACPI S5 Soft Off or hypervisor shutdown port.
    pub fn power_off(&self) -> ! {
        AcpiPowerManager::power_off(self.fadt.as_ref())
    }

    pub fn rsdp(&self) -> Option<&Rsdp> {
        self.rsdp.as_ref()
    }

    pub fn tables(&self) -> &AcpiTables {
        &self.tables
    }

    pub fn fadt(&self) -> Option<&Fadt> {
        self.fadt.as_ref()
    }

    pub fn madt(&self) -> Option<&Madt> {
        self.madt.as_ref()
    }
}

pub static ACPI_MANAGER: IrqSpinlock<Option<AcpiManager>> = IrqSpinlock::new(None);

/// Reboots the system.
pub fn reboot() -> ! {
    let lock = ACPI_MANAGER.lock();
    if let Some(ref mgr) = *lock {
        mgr.reboot();
    } else {
        AcpiPowerManager::reboot(None);
    }
}

/// Powers off the system.
pub fn power_off() -> ! {
    let lock = ACPI_MANAGER.lock();
    if let Some(ref mgr) = *lock {
        mgr.power_off();
    } else {
        AcpiPowerManager::power_off(None);
    }
}

/// Executes a closure with reference to the global `AcpiManager`.
pub fn with_acpi<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AcpiManager) -> R,
{
    let lock = ACPI_MANAGER.lock();
    lock.as_ref().map(f)
}
