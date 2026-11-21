//! x86_64 256-Entry Interrupt Descriptor Table (IDT) & Assembly Entry Stubs
//!
//! Defines the 16-byte 64-bit Interrupt/Trap Gate descriptors, populates all 256
//! IDT vectors (`0..31` CPU exceptions + `32..255` hardware/software interrupt gates),
//! wires Double Fault (`#DF`, vector 8) to TSS `IST1`, and provides the unified
//! `common_interrupt_stub` that preserves the complete 64-bit `CpuState`.

use crate::arch::x86_64::exceptions;
use crate::arch::x86_64::gdt::KERNEL_CODE_SELECTOR;
use crate::arch::x86_64::interrupts;
use crate::arch::x86_64::privilege::{PrivilegeLevel, SegmentSelector};
use crate::arch::x86_64::registers::DescriptorTablePointer;
use crate::arch::x86_64::state::CpuState;
use crate::arch::x86_64::tss::DOUBLE_FAULT_IST_INDEX;
use core::arch::global_asm;
use core::mem::size_of;

/// Total number of architectural IDT entries (`0..=255`).
pub const IDT_ENTRY_COUNT: usize = 256;

/// Expected `IDTR.limit` value (`256 * 16 - 1 = 4095 = 0x0FFF`).
pub const EXPECTED_IDT_LIMIT: u16 = (IDT_ENTRY_COUNT * size_of::<IdtEntry>() - 1) as u16;

/// 16-byte 64-bit IDT Gate Descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,
    pub type_attr: u8,
    pub offset_mid: u16,
    pub offset_high: u32,
    pub reserved: u32,
}

impl IdtEntry {
    /// Creates an absent (zeroed) IDT gate descriptor.
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Configures this IDT entry as a Present 64-bit Interrupt Gate (`0xE`) or Trap Gate (`0xF`).
    pub fn set_handler(
        &mut self,
        handler_addr: u64,
        selector: SegmentSelector,
        dpl: PrivilegeLevel,
        is_trap: bool,
        ist_index: u8,
    ) {
        self.offset_low = (handler_addr & 0xFFFF) as u16;
        self.offset_mid = ((handler_addr >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler_addr >> 32) & 0xFFFF_FFFF) as u32;
        self.selector = selector.0;
        self.ist = ist_index & 0b111;

        let gate_type: u8 = if is_trap { 0x0F } else { 0x0E };
        let dpl_bits: u8 = ((dpl as u8) & 0b11) << 5;
        let present_bit: u8 = 1 << 7;
        self.type_attr = present_bit | dpl_bits | gate_type;
        self.reserved = 0;
    }
}

#[repr(C, align(16))]
struct IdtTable {
    entries: [IdtEntry; IDT_ENTRY_COUNT],
}

impl IdtTable {
    const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); IDT_ENTRY_COUNT],
        }
    }
}

static mut IDT: IdtTable = IdtTable::new();

extern "C" {
    static isr_stub_table: [u64; 48];
    fn isr_stub_128();
    fn isr_stub_255();
}

/// Initializes all 256 descriptors of the 64-bit IDT and loads `IDTR` via `lidt`.
pub fn init() {
    // SAFETY: Populating static IDT during single-core initialization with interrupts disabled.
    let idtr = unsafe {
        // 1. Default all 256 vectors to a safe spurious/unhandled interrupt gate
        let default_addr = isr_stub_255 as *const () as u64;
        let mut i = 0;
        while i < IDT_ENTRY_COUNT {
            IDT.entries[i].set_handler(
                default_addr,
                KERNEL_CODE_SELECTOR,
                PrivilegeLevel::Ring0,
                false,
                0,
            );
            i += 1;
        }

        // 2. Populate vectors 0..31 (CPU Exceptions) and 32..47 (Hardware IRQs 0..15)
        let mut vec = 0usize;
        while vec < 48 {
            let stub_addr = isr_stub_table[vec];
            // Breakpoint (#BP, vector 3) allows Ring 3 DPL and uses Trap Gate semantics
            let dpl = if vec == 3 {
                PrivilegeLevel::Ring3
            } else {
                PrivilegeLevel::Ring0
            };
            let is_trap = vec == 3;
            // Double Fault (#DF, vector 8) switches unconditionally to TSS IST1 stack
            let ist = if vec == 8 {
                DOUBLE_FAULT_IST_INDEX
            } else {
                0
            };

            IDT.entries[vec].set_handler(stub_addr, KERNEL_CODE_SELECTOR, dpl, is_trap, ist);
            vec += 1;
        }

        // 3. Populate vector 0x80 (128) for software system call gate (Ring 3 callable)
        IDT.entries[0x80].set_handler(
            isr_stub_128 as *const () as u64,
            KERNEL_CODE_SELECTOR,
            PrivilegeLevel::Ring3,
            false,
            0,
        );

        DescriptorTablePointer {
            limit: EXPECTED_IDT_LIMIT,
            base: core::ptr::addr_of!(IDT.entries) as u64,
        }
    };

    // SAFETY: Loading validated 256-entry 64-bit IDT via `lidt`.
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) &idtr, options(nostack, preserves_flags));
    }
}

/// Returns the expected base address of the static IDT table.
pub fn idt_base_addr() -> u64 {
    core::ptr::addr_of!(IDT) as u64
}

/// Central Rust interrupt/exception dispatcher invoked from `common_interrupt_stub`.
///
/// Returns the `CpuState` pointer to restore on `iretq`.
#[no_mangle]
pub extern "C" fn interrupt_dispatch(state: &mut CpuState) -> *const CpuState {
    if state.vector < 32 {
        exceptions::dispatch_exception(state);
    } else if (32..=47).contains(&state.vector) {
        interrupts::dispatch_irq(state);
    } else if state.vector == 128 {
        crate::user::handle_user_syscall(state);
    }
    state as *const CpuState
}

global_asm!(
    r#"
    .section .text

    .macro ISR_NOERR vec
    .align 16
    isr_stub_\vec:
        push 0
        push \vec
        jmp common_interrupt_stub
    .endm

    .macro ISR_ERR vec
    .align 16
    isr_stub_\vec:
        push \vec
        jmp common_interrupt_stub
    .endm

    // CPU Exceptions 0..31
    ISR_NOERR 0   // #DE Divide Error
    ISR_NOERR 1   // #DB Debug
    ISR_NOERR 2   // NMI
    ISR_NOERR 3   // #BP Breakpoint
    ISR_NOERR 4   // #OF Overflow
    ISR_NOERR 5   // #BR Bound Range Exceeded
    ISR_NOERR 6   // #UD Invalid Opcode
    ISR_NOERR 7   // #NM Device Not Available
    ISR_ERR   8   // #DF Double Fault (pushes error code 0)
    ISR_NOERR 9   // Coprocessor Segment Overrun
    ISR_ERR   10  // #TS Invalid TSS
    ISR_ERR   11  // #NP Segment Not Present
    ISR_ERR   12  // #SS Stack-Segment Fault
    ISR_ERR   13  // #GP General Protection Fault
    ISR_ERR   14  // #PF Page Fault
    ISR_NOERR 15  // Reserved
    ISR_NOERR 16  // #MF x87 Floating-Point
    ISR_ERR   17  // #AC Alignment Check
    ISR_NOERR 18  // #MC Machine Check
    ISR_NOERR 19  // #XM SIMD Floating-Point
    ISR_NOERR 20  // #VE Virtualization
    ISR_ERR   21  // #CP Control Protection
    ISR_NOERR 22  // Reserved
    ISR_NOERR 23  // Reserved
    ISR_NOERR 24  // Reserved
    ISR_NOERR 25  // Reserved
    ISR_NOERR 26  // Reserved
    ISR_NOERR 27  // Reserved
    ISR_NOERR 28  // #HV Hypervisor Injection
    ISR_ERR   29  // #VC VMM Communication
    ISR_ERR   30  // #SX Security Exception
    ISR_NOERR 31  // Reserved

    // Hardware IRQs 32..47 (PIC / IOAPIC vectors)
    ISR_NOERR 32
    ISR_NOERR 33
    ISR_NOERR 34
    ISR_NOERR 35
    ISR_NOERR 36
    ISR_NOERR 37
    ISR_NOERR 38
    ISR_NOERR 39
    ISR_NOERR 40
    ISR_NOERR 41
    ISR_NOERR 42
    ISR_NOERR 43
    ISR_NOERR 44
    ISR_NOERR 45
    ISR_NOERR 46
    ISR_NOERR 47

    // Software System Call Vector 0x80 (128)
    .global isr_stub_128
    ISR_NOERR 128

    // Default Spurious / Unhandled Interrupt Vector (255)
    .global isr_stub_255
    ISR_NOERR 255

    .align 16
common_interrupt_stub:
    // Save all 15 general-purpose registers matching CpuState layout
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    // Pass RDI = &mut CpuState, align RSP to 16 bytes for System V AMD64 ABI
    mov rdi, rsp
    mov rbp, rsp
    and rsp, -16
    call interrupt_dispatch

    // Restore RSP from returned *const CpuState (supports context switching)
    mov rsp, rax

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    // Discard pushed vector and error_code (16 bytes)
    add rsp, 16
    iretq

    .section .rodata
    .align 8
    .global isr_stub_table
isr_stub_table:
    .quad isr_stub_0,  isr_stub_1,  isr_stub_2,  isr_stub_3
    .quad isr_stub_4,  isr_stub_5,  isr_stub_6,  isr_stub_7
    .quad isr_stub_8,  isr_stub_9,  isr_stub_10, isr_stub_11
    .quad isr_stub_12, isr_stub_13, isr_stub_14, isr_stub_15
    .quad isr_stub_16, isr_stub_17, isr_stub_18, isr_stub_19
    .quad isr_stub_20, isr_stub_21, isr_stub_22, isr_stub_23
    .quad isr_stub_24, isr_stub_25, isr_stub_26, isr_stub_27
    .quad isr_stub_28, isr_stub_29, isr_stub_30, isr_stub_31
    .quad isr_stub_32, isr_stub_33, isr_stub_34, isr_stub_35
    .quad isr_stub_36, isr_stub_37, isr_stub_38, isr_stub_39
    .quad isr_stub_40, isr_stub_41, isr_stub_42, isr_stub_43
    .quad isr_stub_44, isr_stub_45, isr_stub_46, isr_stub_47
"#
);
