//! x86_64 Task Execution Context
//!
//! Represents the callee-preserved register frame saved on the task's stack
//! during a software context switch.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub rflags: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rip: u64,
}

impl TaskContext {
    /// Creates an empty context initialized to zeroes.
    pub const fn empty() -> Self {
        Self {
            rflags: 0x202, // IF=1
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rip: 0,
        }
    }

    /// Initializes a new task context to begin execution at `entry_fn`.
    pub fn new_entry(entry_fn: extern "C" fn()) -> Self {
        Self {
            rflags: 0x202, // IF=1, IOPL=0
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rip: entry_fn as *const () as u64,
        }
    }
}
