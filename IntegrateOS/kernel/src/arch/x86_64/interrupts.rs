//! Hardware Interrupt Dispatcher (IRQs 0..15 / Vectors 32..47)
//!
//! Routes external hardware interrupts to their subsystem drivers, sends End of
//! Interrupt (EOI) acknowledgements to the 8259 PIC, and maintains interrupt accounting.

use crate::arch::x86_64::keyboard;
use crate::arch::x86_64::pic::{self, PIC_MASTER_OFFSET, PIC_SLAVE_OFFSET};
use crate::arch::x86_64::pit;
use crate::arch::x86_64::state::CpuState;
use core::sync::atomic::{AtomicU64, Ordering};

pub const IRQ_TIMER: u8 = 0;
pub const IRQ_KEYBOARD: u8 = 1;
pub const IRQ_CASCADE: u8 = 2;
pub const IRQ_COM2: u8 = 3;
pub const IRQ_COM1: u8 = 4;
pub const IRQ_FLOPPY: u8 = 6;
pub const IRQ_SPURIOUS7: u8 = 7;
pub const IRQ_RTC: u8 = 8;
pub const IRQ_MOUSE: u8 = 12;
pub const IRQ_ATA_PRIMARY: u8 = 14;
pub const IRQ_ATA_SECONDARY: u8 = 15;

static TOTAL_INTERRUPTS: AtomicU64 = AtomicU64::new(0);

/// Initializes the hardware interrupt subsystem: remaps the 8259 PIC, programs the
/// 8254 PIT (100 Hz), unmasks Timer (IRQ 0) and Keyboard (IRQ 1), and leaves others masked.
pub fn init() {
    pic::init(PIC_MASTER_OFFSET, PIC_SLAVE_OFFSET);
    pic::disable_all();

    pit::init(100);

    // Unmask timer (IRQ 0) and keyboard (IRQ 1)
    pic::unmask_irq(IRQ_TIMER);
    pic::unmask_irq(IRQ_KEYBOARD);
}

/// Dispatches an incoming hardware interrupt on vector `32..=47`.
pub fn dispatch_irq(state: &mut CpuState) {
    let irq = (state.vector - (PIC_MASTER_OFFSET as u64)) as u8;
    TOTAL_INTERRUPTS.fetch_add(1, Ordering::Relaxed);

    match irq {
        IRQ_TIMER => {
            pit::handle_timer_interrupt();
            pic::send_eoi(IRQ_TIMER);
            crate::scheduler::check_timer_tick();
            return;
        }
        IRQ_KEYBOARD => {
            keyboard::handle_keyboard_interrupt();
        }
        IRQ_MOUSE => {
            crate::input::mouse::handle_mouse_interrupt();
        }
        _ => {
            // Unhandled IRQ
        }
    }

    pic::send_eoi(irq);
}

/// Returns the total number of hardware interrupts handled since boot.
pub fn get_total_interrupts() -> u64 {
    TOTAL_INTERRUPTS.load(Ordering::Relaxed)
}
