//! Central Process Table & Lifecycle Manager
//!
//! Provides process registration, state transitions, lookup by PID, and lifecycle tracking.

use alloc::vec::Vec;

use crate::process::pcb::ProcessControlBlock;
use crate::process::pid::{next_pid, reset_pid_allocator, ProcessId};
use crate::process::state::ProcessState;
use crate::sync::IrqSpinlock;

pub struct ProcessManager {
    processes: Vec<ProcessControlBlock>,
    current_pid: ProcessId,
}

impl ProcessManager {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current_pid: ProcessId::KERNEL,
        }
    }

    /// Initializes the process manager and creates the root kernel bootstrap process (PID 0).
    pub fn init(&mut self) -> Result<(), &'static str> {
        reset_pid_allocator(0);
        let pid0 = next_pid(); // PID 0
        let idle_pcb = ProcessControlBlock::current_kernel_bootstrap(pid0, "kernel_bootstrap");
        self.processes.push(idle_pcb);
        self.current_pid = pid0;
        Ok(())
    }

    /// Spawns a new kernel thread with dedicated stack and returns its unique PID.
    pub fn spawn_kernel(
        &mut self,
        name: &str,
        entry_fn: extern "C" fn(),
        priority: u8,
    ) -> ProcessId {
        let pid = next_pid();
        let pcb = ProcessControlBlock::new_kernel(pid, self.current_pid, name, entry_fn, priority);
        self.processes.push(pcb);
        pid
    }

    /// Spawns a new user process with isolated address space and returns its PID.
    pub fn spawn_user(
        &mut self,
        name: &str,
        entry_rip: u64,
        user_stack_top: u64,
        cr3: u64,
        priority: u8,
    ) -> ProcessId {
        let pid = next_pid();
        let pcb = ProcessControlBlock::new_user(
            pid,
            self.current_pid,
            name,
            entry_rip,
            user_stack_top,
            cr3,
            priority,
        );
        self.processes.push(pcb);
        pid
    }

    /// Returns a reference to the process with `pid`.
    pub fn get(&self, pid: ProcessId) -> Option<&ProcessControlBlock> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    /// Returns a mutable reference to the process with `pid`.
    pub fn get_mut(&mut self, pid: ProcessId) -> Option<&mut ProcessControlBlock> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }

    /// Returns a reference to the currently executing process.
    pub fn current(&self) -> Option<&ProcessControlBlock> {
        self.get(self.current_pid)
    }

    /// Returns a mutable reference to the currently executing process.
    pub fn current_mut(&mut self) -> Option<&mut ProcessControlBlock> {
        let pid = self.current_pid;
        self.get_mut(pid)
    }

    /// Returns the currently active PID.
    pub fn current_pid(&self) -> ProcessId {
        self.current_pid
    }

    /// Sets the currently active PID.
    pub fn set_current_pid(&mut self, pid: ProcessId) {
        self.current_pid = pid;
    }

    /// Updates the state of the process identified by `pid`.
    pub fn set_state(&mut self, pid: ProcessId, state: ProcessState) {
        if let Some(proc) = self.get_mut(pid) {
            proc.set_state(state);
        }
    }

    /// Terminates a process with the specified exit code.
    pub fn terminate(&mut self, pid: ProcessId, exit_code: i32) {
        self.set_state(pid, ProcessState::Terminated(exit_code));
    }

    /// Returns the total number of processes currently tracked in the process table.
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    /// Returns the number of processes matching a state condition.
    pub fn count_by_state(&self, predicate: fn(&ProcessState) -> bool) -> usize {
        self.processes.iter().filter(|p| predicate(&p.state)).count()
    }

    /// Returns an immutable iterator over all processes.
    pub fn iter(&self) -> core::slice::Iter<'_, ProcessControlBlock> {
        self.processes.iter()
    }
}

pub static PROCESS_MANAGER: IrqSpinlock<ProcessManager> =
    IrqSpinlock::new(ProcessManager::new());
