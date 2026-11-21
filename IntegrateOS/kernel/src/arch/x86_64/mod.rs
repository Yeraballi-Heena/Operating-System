//! x86_64 CPU Architecture Layer (`kernel/src/arch/x86_64/`)
//!
//! Exports:
//! - `privilege`: Ring 0..3 (`PrivilegeLevel`) and `SegmentSelector`
//! - `registers`: `Cr0`, `Cr2`, `Cr3`, `Cr4`, `Msr`, `RFlags`, `CpuInfo`, `sgdt`, `sidt`, segment readers
//! - `state`: `CpuState` and `HardwareInterruptFrame`
//! - `tss`: 64-bit `TaskStateSegment`, `RSP0`, and `IST1..IST2` dedicated stacks
//! - `gdt`: 64-bit Global Descriptor Table (Kernel Ring 0 + User Ring 3 + 16-byte TSS)
//! - `idt`: 256-entry 64-bit Interrupt Descriptor Table & assembly entry stubs
//! - `exceptions`: CPU exception handlers (`#DE`, `#BP`, `#UD`, `#DF`, `#GP`, `#PF`) & self-tests

pub mod exceptions;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod io;
pub mod keyboard;
pub mod pic;
pub mod pit;
pub mod privilege;
pub mod registers;
pub mod state;
pub mod tss;

use crate::arch::x86_64::gdt::{
    EXPECTED_GDT_LIMIT, KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR, TSS_SELECTOR,
    USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};
use crate::arch::x86_64::idt::EXPECTED_IDT_LIMIT;
use crate::arch::x86_64::privilege::PrivilegeLevel;
use crate::arch::x86_64::registers::{
    current_privilege_level, read_cs, read_ds, read_ss, read_tr, sgdt, sidt, CpuInfo, Cr0, Cr3,
    Cr4, Msr,
};
use crate::{klog_info, klog_ok};

/// Initializes the x86_64 CPU architecture layer (TSS, GDT, segment selectors, Task Register, IDT)
/// and verifies the live processor state.
pub fn init() {
    gdt::init();
    idt::init();
    verify_cpu_architecture();
}

/// Reads back the live hardware registers (`SGDT`, `SIDT`, `CS`, `DS`, `SS`, `TR`, `CR0`, `CR4`, `EFER`)
/// and asserts that the processor is running with the expected architectural state.
pub fn verify_cpu_architecture() {
    let gdtr = sgdt();
    let gdtr_limit = gdtr.limit;
    let gdtr_base = gdtr.base;
    let expected_gdt_base = gdt::gdt_base_addr();

    if gdtr_limit != EXPECTED_GDT_LIMIT || gdtr_base != expected_gdt_base {
        panic!(
            "GDTR verification failed: got (base={:#x}, limit={}), expected (base={:#x}, limit={})",
            gdtr_base, gdtr_limit, expected_gdt_base, EXPECTED_GDT_LIMIT
        );
    }

    let idtr = sidt();
    let idtr_limit = idtr.limit;
    let idtr_base = idtr.base;
    let expected_idt_base = idt::idt_base_addr();

    if idtr_limit != EXPECTED_IDT_LIMIT || idtr_base != expected_idt_base {
        panic!(
            "IDTR verification failed: got (base={:#x}, limit={:#x}), expected (base={:#x}, limit={:#x})",
            idtr_base, idtr_limit, expected_idt_base, EXPECTED_IDT_LIMIT
        );
    }

    let cs = read_cs();
    let ds = read_ds();
    let ss = read_ss();
    let tr = read_tr();
    let cpl = current_privilege_level();

    if cs != KERNEL_CODE_SELECTOR {
        panic!(
            "CS verification failed: got {:#06x}, expected {:#06x}",
            cs.0, KERNEL_CODE_SELECTOR.0
        );
    }
    if ds != KERNEL_DATA_SELECTOR || ss != KERNEL_DATA_SELECTOR {
        panic!(
            "DS/SS verification failed: DS={:#06x}, SS={:#06x}, expected {:#06x}",
            ds.0, ss.0, KERNEL_DATA_SELECTOR.0
        );
    }
    if tr != TSS_SELECTOR {
        panic!(
            "TSS Task Register (TR) verification failed: got {:#06x}, expected {:#06x}",
            tr.0, TSS_SELECTOR.0
        );
    }
    if cpl != PrivilegeLevel::Ring0 {
        panic!("Expected Kernel Mode (Ring0), found {:?}", cpl);
    }

    let cr0 = Cr0::read();
    let cr4 = Cr4::read();
    let efer = Msr::IA32_EFER.read();

    if !cr0.contains(Cr0::PROTECTED_MODE_ENABLE | Cr0::PAGING_ENABLE | Cr0::WRITE_PROTECT) {
        panic!("CR0 missing required flags (PE | PG | WP): {:#018x}", cr0.0);
    }
    if !cr4.contains(Cr4::PHYSICAL_ADDRESS_EXTENSION) {
        panic!("CR4 missing PAE flag: {:#018x}", cr4.0);
    }
    if (efer & (Msr::EFER_LONG_MODE_ENABLE | Msr::EFER_LONG_MODE_ACTIVE))
        != (Msr::EFER_LONG_MODE_ENABLE | Msr::EFER_LONG_MODE_ACTIVE)
    {
        panic!("IA32_EFER missing LME/LMA flags: {:#018x}", efer);
    }
}

/// Logs the verified x86_64 CPU, GDT, TSS, IDT, and Control Register state to VGA and Serial.
pub fn log_architecture_state() {
    let cpu = CpuInfo::query();
    let gdtr = sgdt();
    let gdtr_base = gdtr.base;
    let gdtr_limit = gdtr.limit;
    let idtr = sidt();
    let idtr_base = idtr.base;
    let idtr_limit = idtr.limit;
    let cs = read_cs();
    let ds = read_ds();
    let ss = read_ss();
    let tr = read_tr();
    let (rsp0, ist1, ist2) = tss::configured_stacks();
    let cr0 = Cr0::read();
    let cr3 = Cr3::read();
    let cr4 = Cr4::read();
    let efer = Msr::IA32_EFER.read();

    klog_ok!(
        "CPU Identified: {} (Family {}, Model {}, Stepping {}, LM={}, NX={})",
        cpu.vendor_str(),
        cpu.family,
        cpu.model,
        cpu.stepping,
        cpu.has_long_mode,
        cpu.has_nx
    );
    klog_ok!(
        "GDT Initialized: base={:#010x}, limit=0x{:02x} (KCODE={:#04x} KDATA={:#04x} UDATA={:#04x} UCODE={:#04x})",
        gdtr_base,
        gdtr_limit,
        KERNEL_CODE_SELECTOR.0,
        KERNEL_DATA_SELECTOR.0,
        USER_DATA_SELECTOR.0,
        USER_CODE_SELECTOR.0
    );
    klog_ok!(
        "TSS Verified  : TR={:#04x} RSP0={:#010x} IST1(#DF)={:#010x} IST2={:#010x}",
        tr.0,
        rsp0,
        ist1,
        ist2
    );
    klog_ok!(
        "IDT Initialized: base={:#010x}, limit=0x{:04x} (256 gates, #DF->IST1)",
        idtr_base,
        idtr_limit
    );
    klog_info!(
        "Control Regs  : CPL=Ring0 CS={:#04x} DS={:#04x} SS={:#04x} CR0={:#010x} CR3={:#010x} CR4={:#04x} EFER={:#05x}",
        cs.0,
        ds.0,
        ss.0,
        cr0.0,
        cr3.0,
        cr4.0,
        efer
    );
}
