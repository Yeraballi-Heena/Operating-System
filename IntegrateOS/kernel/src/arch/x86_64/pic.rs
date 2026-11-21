//! 8259 Dual Programmable Interrupt Controller (PIC) Driver
//!
//! Conceptual Reference: The Little Book About Operating System Development
//! Hardware IRQs 0..15 are remapped away from legacy 0x08..0x0F (which conflict
//! with x86 CPU exceptions 0..31) to vectors 32..47:
//! - Master PIC: IRQ 0..7  -> Vectors 32..39 (0x20..0x27)
//! - Slave PIC:  IRQ 8..15 -> Vectors 40..47 (0x28..0x2F)

use crate::serial::{inb, outb};

pub const PIC1_COMMAND: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_COMMAND: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;

pub const PIC_EOI: u8 = 0x20;

// Initialization Command Words (ICW)
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

pub const PIC_MASTER_OFFSET: u8 = 32;
pub const PIC_SLAVE_OFFSET: u8 = 40;

/// Pauses briefly for legacy I/O bus operations (port 0x80 write).
#[inline(always)]
fn io_wait() {
    // SAFETY: Writing to port 0x80 is standard delay practice on x86 hardware.
    unsafe {
        outb(0x80, 0x00);
    }
}

/// Remaps the 8259 Master & Slave PICs to base interrupt vectors `offset1` (32) and `offset2` (40),
/// masks all IRQ lines initially, and clears pending interrupts.
pub fn init(offset1: u8, offset2: u8) {
    // SAFETY: Initializing 8259 PIC registers via standard hardware outb sequence.
    unsafe {
        // Save current interrupt masks
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // Start initialization sequence in cascade mode (ICW1)
        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();

        // ICW2: Vector offsets
        outb(PIC1_DATA, offset1);
        io_wait();
        outb(PIC2_DATA, offset2);
        io_wait();

        // ICW3: Master-Slave cascading
        outb(PIC1_DATA, 0x04); // Master has slave on IRQ2 (0000 0100)
        io_wait();
        outb(PIC2_DATA, 0x02); // Slave connected to Master's IRQ2 (cascade identity 2)
        io_wait();

        // ICW4: 8086/88 mode
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Restore masks (or mask all if desired)
        outb(PIC1_DATA, mask1);
        outb(PIC2_DATA, mask2);
    }
}

/// Sends End of Interrupt (EOI) signal for hardware `irq` (0..15).
pub fn send_eoi(irq: u8) {
    // SAFETY: Acknowledging hardware IRQ via PIC command registers.
    unsafe {
        if irq >= 8 {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_COMMAND, PIC_EOI);
    }
}

/// Enables (unmasks) a specific hardware IRQ line (0..15).
pub fn unmask_irq(irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let bit = if irq < 8 { irq } else { irq - 8 };
    // SAFETY: Modifying PIC data register bitmask.
    unsafe {
        let value = inb(port) & !(1 << bit);
        outb(port, value);
    }
}

/// Disables (masks) a specific hardware IRQ line (0..15).
pub fn mask_irq(irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let bit = if irq < 8 { irq } else { irq - 8 };
    // SAFETY: Modifying PIC data register bitmask.
    unsafe {
        let value = inb(port) | (1 << bit);
        outb(port, value);
    }
}

/// Disables all hardware IRQ lines on both Master and Slave PICs.
pub fn disable_all() {
    // SAFETY: Masking all 16 IRQs.
    unsafe {
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
    }
}
