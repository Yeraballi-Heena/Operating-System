//! Process Control Block (PCB) Architecture
//!
//! Encapsulates full process metadata, hardware register context, virtual address space (CR3),
//! kernel stack allocation, and lifecycle statistics.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::arch::x86_64::registers::Cr3;
use crate::process::pid::ProcessId;
use crate::process::state::ProcessState;

/// Default size of a kernel thread execution stack (16 KiB).
pub const PROCESS_KERNEL_STACK_SIZE: usize = 16 * 1024;

/// Default CPU affinity bitmask: all 16 logical cores (0..15) allowed (0x0000_FFFF).
pub const DEFAULT_CPU_AFFINITY: u64 = 0x0000_FFFF;

/// Process Control Block (PCB).
pub struct ProcessControlBlock {
    pub pid: ProcessId,
    pub ppid: ProcessId,
    pub name: String,
    pub state: ProcessState,
    pub priority: u8,
    pub cr3: u64,
    pub kernel_stack: Vec<u8>,
    pub kernel_stack_top: u64,
    pub saved_rsp: u64,
    pub user_stack_top: Option<u64>,
    pub entry_point: u64,
    pub ticks_used: u64,
    pub is_user: bool,
    /// Bitmask of permitted logical CPU cores for scheduling affinity.
    pub cpu_affinity: u64,
}

impl ProcessControlBlock {
    /// Creates a new kernel-mode process with its own dedicated 16 KiB kernel stack.
    pub fn new_kernel(
        pid: ProcessId,
        ppid: ProcessId,
        name: &str,
        entry_fn: extern "C" fn(),
        priority: u8,
    ) -> Self {
        let kernel_stack = vec![0u8; PROCESS_KERNEL_STACK_SIZE];
        let stack_bottom = kernel_stack.as_ptr() as u64;
        let stack_top = (stack_bottom + PROCESS_KERNEL_STACK_SIZE as u64) & !0x0F;

        // Initialize stack frame for `switch_context_asm`:
        // [top - 8]  = kernel_thread_exit_stub (return target if entry_fn returns)
        // [top - 16] = entry_fn (return target for `ret` in switch_context_asm)
        // [top - 24] = rbx (0)
        // [top - 32] = rbp (0)
        // [top - 40] = r12 (0)
        // [top - 48] = r13 (0)
        // [top - 56] = r14 (0)
        // [top - 64] = r15 (0)
        // [top - 72] = rflags (0x202: IF=1)
        let frame_size = 9 * 8; // 72 bytes
        let initial_rsp = stack_top - frame_size;

        // SAFETY: Stack buffer is allocated on heap and aligned to 16 bytes.
        unsafe {
            let frame_ptr = initial_rsp as *mut u64;
            *frame_ptr.add(8) = crate::scheduler::kernel_thread_exit_stub as *const () as u64;
            *frame_ptr.add(7) = entry_fn as *const () as u64; // Return address
            *frame_ptr.add(6) = 0;                             // RBX
            *frame_ptr.add(5) = 0;                             // RBP
            *frame_ptr.add(4) = 0;                             // R12
            *frame_ptr.add(3) = 0;                             // R13
            *frame_ptr.add(2) = 0;                             // R14
            *frame_ptr.add(1) = 0;                             // R15
            *frame_ptr.add(0) = 0x202;                         // RFLAGS (IF=1)
        }

        Self {
            pid,
            ppid,
            name: String::from(name),
            state: ProcessState::Ready,
            priority,
            cr3: Cr3::read().pml4_phys_addr(),
            kernel_stack,
            kernel_stack_top: stack_top,
            saved_rsp: initial_rsp,
            user_stack_top: None,
            entry_point: entry_fn as *const () as u64,
            ticks_used: 0,
            is_user: false,
            cpu_affinity: DEFAULT_CPU_AFFINITY,
        }
    }

    /// Creates a PCB representing the currently running bootstrap kernel task (PID 0).
    pub fn current_kernel_bootstrap(pid: ProcessId, name: &str) -> Self {
        Self {
            pid,
            ppid: ProcessId::KERNEL,
            name: String::from(name),
            state: ProcessState::Running,
            priority: 0,
            cr3: Cr3::read().pml4_phys_addr(),
            kernel_stack: Vec::new(), // Bootstrap stack is statically managed in .bss.stack
            kernel_stack_top: 0,
            saved_rsp: 0,
            user_stack_top: None,
            entry_point: 0,
            ticks_used: 0,
            is_user: false,
            cpu_affinity: DEFAULT_CPU_AFFINITY,
        }
    }

    /// Creates a user-mode process with isolated address space and stack.
    pub fn new_user(
        pid: ProcessId,
        ppid: ProcessId,
        name: &str,
        entry_rip: u64,
        user_stack_top: u64,
        cr3: u64,
        priority: u8,
    ) -> Self {
        let kernel_stack = vec![0u8; PROCESS_KERNEL_STACK_SIZE];
        let stack_bottom = kernel_stack.as_ptr() as u64;
        let stack_top = (stack_bottom + PROCESS_KERNEL_STACK_SIZE as u64) & !0x0F;

        Self {
            pid,
            ppid,
            name: String::from(name),
            state: ProcessState::Ready,
            priority,
            cr3,
            kernel_stack,
            kernel_stack_top: stack_top,
            saved_rsp: stack_top,
            user_stack_top: Some(user_stack_top),
            entry_point: entry_rip,
            ticks_used: 0,
            is_user: true,
            cpu_affinity: DEFAULT_CPU_AFFINITY,
        }
    }

    /// Transitions the process to a new lifecycle state.
    #[inline]
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    /// Returns true if this process is ready or currently running.
    #[inline(always)]
    pub fn is_runnable(&self) -> bool {
        self.state.is_runnable()
    }

    /// Sets the CPU affinity mask for this process.
    pub fn set_cpu_affinity(&mut self, mask: u64) {
        self.cpu_affinity = mask;
    }

    /// Returns the CPU affinity mask for this process.
    pub fn get_cpu_affinity(&self) -> u64 {
        self.cpu_affinity
    }

    /// Returns true if execution is permitted on logical CPU `cpu_id`.
    pub fn is_cpu_allowed(&self, cpu_id: usize) -> bool {
        if cpu_id < 64 {
            (self.cpu_affinity & (1u64 << cpu_id)) != 0
        } else {
            false
        }
    }
}
