//! x86_64 64-Bit Task State Segment (TSS)
//!
//! In 64-bit Long Mode, hardware task switching is not used, but the TSS is
//! mandatory for two critical hardware mechanisms:
//! 1. `privilege_stack_table[0]` (`RSP0`): The kernel stack pointer loaded by the
//!    CPU when transitioning from User Mode (Ring 3) to Kernel Mode (Ring 0) on
//!    any interrupt, exception, or syscall.
//! 2. `interrupt_stack_table[0..7]` (`IST1..IST7`): Dedicated known-good kernel
//!    stacks switched to unconditionally on critical exceptions such as Double
//!    Fault (`#DF`), preventing Triple Faults on kernel stack overflow.

use core::mem::size_of;

/// IST index (1-based: `1..=7`) used for Double Fault (`#DF`) exception handling.
pub const DOUBLE_FAULT_IST_INDEX: u8 = 1;

/// IST index (1-based: `1..=7`) reserved for critical NMI / fault handling.
pub const CRITICAL_FAULT_IST_INDEX: u8 = 2;

/// Size of each dedicated TSS stack (16 KiB).
pub const TSS_STACK_SIZE: usize = 16 * 1024;

#[repr(C, align(16))]
struct AlignedStack([u8; TSS_STACK_SIZE]);

static mut PRIVILEGE_RING0_STACK: AlignedStack = AlignedStack([0; TSS_STACK_SIZE]);
static mut DOUBLE_FAULT_IST_STACK: AlignedStack = AlignedStack([0; TSS_STACK_SIZE]);
static mut CRITICAL_FAULT_IST_STACK: AlignedStack = AlignedStack([0; TSS_STACK_SIZE]);

/// Architectural 64-bit Task State Segment (104 bytes).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TaskStateSegment {
    pub reserved_1: u32,
    /// Stack pointers (`RSP0`, `RSP1`, `RSP2`) loaded on privilege transitions from lower rings.
    pub privilege_stack_table: [u64; 3],
    pub reserved_2: u64,
    /// Interrupt Stack Table pointers (`IST1` .. `IST7`).
    pub interrupt_stack_table: [u64; 7],
    pub reserved_3: u64,
    pub reserved_4: u16,
    /// Offset from TSS base to the I/O Permission Bitmap (`>= sizeof(TSS)` disables I/O bitmap).
    pub iomap_base: u16,
}

impl TaskStateSegment {
    /// Creates an uninitialized (zeroed) 64-bit TSS with the I/O permission map disabled.
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            privilege_stack_table: [0; 3],
            reserved_2: 0,
            interrupt_stack_table: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: size_of::<Self>() as u16,
        }
    }
}

static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// Initializes the global 64-bit TSS with dedicated `RSP0`, `IST1`, and `IST2` stack tops
/// and returns `(tss_base_addr, tss_limit)`.
pub fn init() -> (u64, u16) {
    let rsp0_top = (core::ptr::addr_of!(PRIVILEGE_RING0_STACK) as u64) + (TSS_STACK_SIZE as u64);
    let ist1_top = (core::ptr::addr_of!(DOUBLE_FAULT_IST_STACK) as u64) + (TSS_STACK_SIZE as u64);
    let ist2_top = (core::ptr::addr_of!(CRITICAL_FAULT_IST_STACK) as u64) + (TSS_STACK_SIZE as u64);

    // SAFETY: Single-core early architectural initialization with interrupts disabled.
    unsafe {
        TSS.privilege_stack_table[0] = rsp0_top;
        TSS.interrupt_stack_table[(DOUBLE_FAULT_IST_INDEX - 1) as usize] = ist1_top;
        TSS.interrupt_stack_table[(CRITICAL_FAULT_IST_INDEX - 1) as usize] = ist2_top;
        TSS.iomap_base = size_of::<TaskStateSegment>() as u16;
    }

    let base = core::ptr::addr_of!(TSS) as u64;
    let limit = (size_of::<TaskStateSegment>() - 1) as u16;
    (base, limit)
}

/// Updates `TSS.privilege_stack_table[0]` (`RSP0`) for User -> Kernel transitions.
pub fn set_rsp0(new_rsp0: u64) {
    // SAFETY: Atomic/aligned write to RSP0 during context switch with interrupts disabled.
    unsafe {
        let ptr = core::ptr::addr_of_mut!(TSS.privilege_stack_table);
        (*ptr)[0] = new_rsp0;
    }
}

/// Returns the current `(rsp0, ist1, ist2)` stack top addresses configured in the TSS.
pub fn configured_stacks() -> (u64, u64, u64) {
    // SAFETY: Reading static TSS fields initialized during boot.
    unsafe {
        let pst = core::ptr::addr_of!(TSS.privilege_stack_table).read_unaligned();
        let ist = core::ptr::addr_of!(TSS.interrupt_stack_table).read_unaligned();
        (pst[0], ist[0], ist[1])
    }
}
