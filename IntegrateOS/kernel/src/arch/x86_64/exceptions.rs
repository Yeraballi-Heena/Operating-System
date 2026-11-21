//! x86_64 CPU Exception Dispatch & Diagnostic State Reporting (Vectors `0..31`)
//!
//! Handles all 32 architectural x86_64 CPU exceptions:
//! - `#DE` (Vector 0)  : Divide Error
//! - `#DB` (Vector 1)  : Debug Exception
//! - `NMI` (Vector 2)  : Non-Maskable Interrupt
//! - `#BP` (Vector 3)  : Breakpoint (`int3`)
//! - `#OF` (Vector 4)  : Overflow
//! - `#BR` (Vector 5)  : Bound Range Exceeded
//! - `#UD` (Vector 6)  : Invalid Opcode (`ud2`)
//! - `#NM` (Vector 7)  : Device Not Available
//! - `#DF` (Vector 8)  : Double Fault (runs on dedicated TSS `IST1` stack)
//! - `#TS` (Vector 10) : Invalid TSS
//! - `#NP` (Vector 11) : Segment Not Present
//! - `#SS` (Vector 12) : Stack-Segment Fault
//! - `#GP` (Vector 13) : General Protection Fault
//! - `#PF` (Vector 14) : Page Fault (reads `CR2` linear address & decodes error code)
//! - Vectors `15..31`  : x87/Alignment/Machine-Check/SIMD/Security exceptions

use crate::arch::x86_64::registers::{Cr0, Cr2, Cr3, Cr4};
use crate::arch::x86_64::state::CpuState;
use crate::vga::{self, Color};
use crate::{halt_loop, klog_excp, klog_ok, kprintln};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const VEC_DIVIDE_ERROR: u8 = 0;
pub const VEC_DEBUG: u8 = 1;
pub const VEC_NMI: u8 = 2;
pub const VEC_BREAKPOINT: u8 = 3;
pub const VEC_OVERFLOW: u8 = 4;
pub const VEC_BOUND_RANGE: u8 = 5;
pub const VEC_INVALID_OPCODE: u8 = 6;
pub const VEC_DEVICE_NOT_AVAILABLE: u8 = 7;
pub const VEC_DOUBLE_FAULT: u8 = 8;
pub const VEC_INVALID_TSS: u8 = 10;
pub const VEC_SEGMENT_NOT_PRESENT: u8 = 11;
pub const VEC_STACK_SEGMENT: u8 = 12;
pub const VEC_GENERAL_PROTECTION: u8 = 13;
pub const VEC_PAGE_FAULT: u8 = 14;
pub const VEC_X87_FPU: u8 = 16;
pub const VEC_ALIGNMENT_CHECK: u8 = 17;
pub const VEC_MACHINE_CHECK: u8 = 18;
pub const VEC_SIMD_FP: u8 = 19;
pub const VEC_VIRTUALIZATION: u8 = 20;
pub const VEC_CONTROL_PROTECTION: u8 = 21;

static SAFE_EXCEPTION_TEST_ACTIVE: AtomicBool = AtomicBool::new(false);
static BREAKPOINT_COUNT: AtomicU32 = AtomicU32::new(0);
static DIVIDE_ERROR_TEST_COUNT: AtomicU32 = AtomicU32::new(0);
static INVALID_OPCODE_TEST_COUNT: AtomicU32 = AtomicU32::new(0);

pub static USER_TEST_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static USER_GP_CAUGHT: AtomicU32 = AtomicU32::new(0);
pub static USER_PF_CAUGHT: AtomicU32 = AtomicU32::new(0);

/// Returns the architectural mnemonic and description for CPU exception `vector` (`0..31`).
pub const fn exception_name(vector: u8) -> (&'static str, &'static str) {
    match vector {
        0 => ("#DE", "Divide Error"),
        1 => ("#DB", "Debug Exception"),
        2 => ("NMI", "Non-Maskable Interrupt"),
        3 => ("#BP", "Breakpoint"),
        4 => ("#OF", "Overflow"),
        5 => ("#BR", "Bound Range Exceeded"),
        6 => ("#UD", "Invalid Opcode"),
        7 => ("#NM", "Device Not Available"),
        8 => ("#DF", "Double Fault"),
        9 => ("#CSO", "Coprocessor Segment Overrun"),
        10 => ("#TS", "Invalid TSS"),
        11 => ("#NP", "Segment Not Present"),
        12 => ("#SS", "Stack-Segment Fault"),
        13 => ("#GP", "General Protection Fault"),
        14 => ("#PF", "Page Fault"),
        16 => ("#MF", "x87 Floating-Point Exception"),
        17 => ("#AC", "Alignment Check"),
        18 => ("#MC", "Machine Check"),
        19 => ("#XM", "SIMD Floating-Point Exception"),
        20 => ("#VE", "Virtualization Exception"),
        21 => ("#CP", "Control Protection Exception"),
        29 => ("#VC", "VMM Communication Exception"),
        30 => ("#SX", "Security Exception"),
        _ => ("#??", "Reserved CPU Exception"),
    }
}

/// Primary CPU exception dispatcher for vectors `0..31`.
pub fn dispatch_exception(state: &mut CpuState) {
    let vector = state.vector as u8;
    match vector {
        VEC_DIVIDE_ERROR => handle_divide_error(state),
        VEC_BREAKPOINT => handle_breakpoint(state),
        VEC_INVALID_OPCODE => handle_invalid_opcode(state),
        VEC_DOUBLE_FAULT => handle_double_fault(state),
        VEC_GENERAL_PROTECTION => handle_general_protection_fault(state),
        VEC_PAGE_FAULT => handle_page_fault(state),
        _ => {
            let (mnemonic, title) = exception_name(vector);
            dump_fatal_exception(mnemonic, title, state);
        }
    }
}

/// Vector 3 (`#BP`): Breakpoint exception handler (`int3`).
fn handle_breakpoint(state: &mut CpuState) {
    let count = BREAKPOINT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    klog_excp!(
        "#BP Breakpoint (vec=3, count={}): RIP={:#010x} CS={:#04x} RFLAGS={:#06x} RSP={:#010x}",
        count,
        state.rip,
        state.cs,
        state.rflags,
        state.rsp
    );
}

/// Vector 0 (`#DE`): Divide Error exception handler.
fn handle_divide_error(state: &mut CpuState) {
    if SAFE_EXCEPTION_TEST_ACTIVE.load(Ordering::Acquire) {
        let insn_ptr = state.rip as *const u8;
        // SAFETY: Faulting RIP points to the 2-byte `div ecx` (0xF7 0xF1) test instruction.
        let (b0, b1) = unsafe { (*insn_ptr, *insn_ptr.add(1)) };
        if b0 == 0xF7 && b1 == 0xF1 {
            DIVIDE_ERROR_TEST_COUNT.fetch_add(1, Ordering::Relaxed);
            klog_excp!(
                "#DE Divide Error (vec=0): caught `div ecx` (0xF7 0xF1) at RIP={:#010x}, advancing RIP+2",
                state.rip
            );
            state.rip += 2;
            return;
        }
    }
    dump_fatal_exception("#DE", "Divide Error (Division by Zero / Overflow)", state);
}

/// Vector 6 (`#UD`): Invalid Opcode exception handler.
fn handle_invalid_opcode(state: &mut CpuState) {
    if SAFE_EXCEPTION_TEST_ACTIVE.load(Ordering::Acquire) {
        let insn_ptr = state.rip as *const u8;
        // SAFETY: Faulting RIP points to the 2-byte `ud2` (0x0F 0x0B) test instruction.
        let (b0, b1) = unsafe { (*insn_ptr, *insn_ptr.add(1)) };
        if b0 == 0x0F && b1 == 0x0B {
            INVALID_OPCODE_TEST_COUNT.fetch_add(1, Ordering::Relaxed);
            klog_excp!(
                "#UD Invalid Opcode (vec=6): caught `ud2` (0x0F 0x0B) at RIP={:#010x}, advancing RIP+2",
                state.rip
            );
            state.rip += 2;
            return;
        }
    }
    dump_fatal_exception("#UD", "Invalid Opcode", state);
}

/// Vector 8 (`#DF`): Double Fault handler executing on dedicated TSS `IST1` stack.
fn handle_double_fault(state: &mut CpuState) -> ! {
    dump_fatal_exception("#DF", "Double Fault (Dedicated IST1 Stack)", state)
}

/// Vector 13 (`#GP`): General Protection Fault handler with selector error decoding.
fn handle_general_protection_fault(state: &mut CpuState) {
    if (state.cs & 3) == 3 && USER_TEST_ACTIVE.load(Ordering::Acquire) {
        let insn_ptr = state.rip as *const u8;
        // SAFETY: Reading user-mode code byte at faulting RIP.
        let opcode = unsafe { *insn_ptr };
        if opcode == 0xFA {
            // `cli` instruction in Ring 3
            USER_GP_CAUGHT.fetch_add(1, Ordering::Relaxed);
            klog_excp!(
                "#GP Privileged Instruction Violation (vec=13): caught `cli` (0xFA) in Ring 3 at RIP={:#010x}, advancing RIP+1",
                state.rip
            );
            state.rip += 1;
            return;
        }
    }

    let err = state.error_code;
    let ext = (err & 1) != 0;
    let tbl = match (err >> 1) & 0b11 {
        0b00 => "GDT",
        0b01 | 0b11 => "IDT",
        0b10 => "LDT",
        _ => "???",
    };
    let index = (err >> 3) & 0x1FFF;

    vga::set_color(Color::LightRed, Color::Black);
    kprintln!(
        "\n[#GP DIAGNOSTIC] Error={:#06x} (Table={}, Index={}, External={})",
        err,
        tbl,
        index,
        ext
    );
    dump_fatal_exception("#GP", "General Protection Fault", state)
}

/// Vector 14 (`#PF`): Page Fault handler with `CR2` linear address and error code decoding.
fn handle_page_fault(state: &mut CpuState) {
    let fault_addr = Cr2::read();
    let err = state.error_code;
    let present = (err & (1 << 0)) != 0;
    let write = (err & (1 << 1)) != 0;
    let user = (err & (1 << 2)) != 0;
    let rsvd = (err & (1 << 3)) != 0;
    let instr = (err & (1 << 4)) != 0;

    if (state.cs & 3) == 3 && user && USER_TEST_ACTIVE.load(Ordering::Acquire) {
        if fault_addr == 0x0010_0000 {
            USER_PF_CAUGHT.fetch_add(1, Ordering::Relaxed);
            klog_excp!(
                "#PF Supervisor Memory Violation (vec=14): caught Ring 3 access to CR2={:#018x} (Kernel Code), advancing RIP+7",
                fault_addr
            );
            // Advance past 7-byte instruction `mov eax, [0x00100000]`
            state.rip += 7;
            return;
        }
    }

    vga::set_color(Color::LightRed, Color::Black);
    kprintln!(
        "\n[#PF DIAGNOSTIC] CR2={:#018x} Error={:#04x} (P={} W={} U={} RSVD={} IF={})",
        fault_addr,
        err,
        present as u8,
        write as u8,
        user as u8,
        rsvd as u8,
        instr as u8
    );
    dump_fatal_exception("#PF", "Page Fault", state)
}

/// Prints a complete diagnostic dump of the interrupted `CpuState` and control registers, then halts.
pub fn dump_fatal_exception(mnemonic: &str, title: &str, state: &CpuState) -> ! {
    vga::set_color(Color::White, Color::Red);
    kprintln!("\n========================================================");
    kprintln!(
        "  CPU EXCEPTION {} (Vector {}) — {}",
        mnemonic,
        state.vector,
        title
    );
    kprintln!("========================================================");
    vga::set_color(Color::LightRed, Color::Black);
    kprintln!(
        "  RIP={:#018x}  CS={:#06x}  RFLAGS={:#010x}  CPL={:?}",
        state.rip,
        state.cs,
        state.rflags,
        state.privilege_level()
    );
    kprintln!(
        "  RSP={:#018x}  SS={:#06x}  ERR   ={:#018x}",
        state.rsp,
        state.ss,
        state.error_code
    );
    kprintln!(
        "  RAX={:#018x}  RBX={:#018x}  RCX={:#018x}",
        state.rax,
        state.rbx,
        state.rcx
    );
    kprintln!(
        "  RDX={:#018x}  RSI={:#018x}  RDI={:#018x}",
        state.rdx,
        state.rsi,
        state.rdi
    );
    kprintln!(
        "  RBP={:#018x}  R8 ={:#018x}  R9 ={:#018x}",
        state.rbp,
        state.r8,
        state.r9
    );
    kprintln!(
        "  R10={:#018x}  R11={:#018x}  R12={:#018x}",
        state.r10,
        state.r11,
        state.r12
    );
    kprintln!(
        "  R13={:#018x}  R14={:#018x}  R15={:#018x}",
        state.r13,
        state.r14,
        state.r15
    );
    kprintln!(
        "  CR0={:#010x}  CR2={:#018x}  CR3={:#010x}  CR4={:#06x}",
        Cr0::read().0,
        Cr2::read(),
        Cr3::read().0,
        Cr4::read().0
    );
    halt_loop()
}

/// Runs intentional, non-destructive CPU exception self-tests (`#BP`, `#DE`, `#UD`)
/// and verifies that all three exception handlers execute and return cleanly.
pub fn run_exception_self_tests() {
    SAFE_EXCEPTION_TEST_ACTIVE.store(true, Ordering::Release);

    // 1. Trigger Breakpoint Exception (#BP, Vector 3) via `int3`
    // SAFETY: #BP trap handler logs the exception and returns cleanly via iretq.
    unsafe {
        core::arch::asm!("int3", options(nomem, nostack));
    }

    // 2. Trigger Divide Error Exception (#DE, Vector 0) via `div ecx` with ECX = 0
    // SAFETY: In self-test mode, #DE handler verifies opcode `0xF7 0xF1` and advances RIP+2.
    unsafe {
        core::arch::asm!(
            "mov eax, 42",
            "xor edx, edx",
            "xor ecx, ecx",
            ".byte 0xF7, 0xF1", // `div ecx` (2 bytes)
            out("eax") _,
            out("edx") _,
            out("ecx") _,
            options(nomem, nostack)
        );
    }

    // 3. Trigger Invalid Opcode Exception (#UD, Vector 6) via `ud2` (`0x0F 0x0B`)
    // SAFETY: In self-test mode, #UD handler verifies opcode `0x0F 0x0B` and advances RIP+2.
    unsafe {
        core::arch::asm!("ud2", options(nomem, nostack));
    }

    SAFE_EXCEPTION_TEST_ACTIVE.store(false, Ordering::Release);

    let bp = BREAKPOINT_COUNT.load(Ordering::Relaxed);
    let de = DIVIDE_ERROR_TEST_COUNT.load(Ordering::Relaxed);
    let ud = INVALID_OPCODE_TEST_COUNT.load(Ordering::Relaxed);

    if bp != 1 || de != 1 || ud != 1 {
        panic!(
            "CPU exception self-test mismatch: #BP={}, #DE={}, #UD={} (expected 1, 1, 1)",
            bp, de, ud
        );
    }

    klog_ok!(
        "Exception Self-Tests Passed: #BP={}, #DE={}, #UD={} recovered cleanly",
        bp,
        de,
        ud
    );
}
