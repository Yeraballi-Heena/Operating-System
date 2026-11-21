//! Task Wait Queue
//!
//! Provides FIFO queueing for tasks waiting on condition variables, I/O events,
//! or kernel synchronization primitives.

use alloc::vec::Vec;

use crate::process::manager::PROCESS_MANAGER;
use crate::process::pid::ProcessId;
use crate::sync::spinlock::IrqSpinlock;

/// A thread wait queue holding tasks blocked on an event.
pub struct WaitQueue {
    queue: IrqSpinlock<Vec<ProcessId>>,
}

impl WaitQueue {
    /// Creates an empty wait queue.
    pub const fn new() -> Self {
        Self {
            queue: IrqSpinlock::new(Vec::new()),
        }
    }

    /// Enqueues a process into the wait queue.
    pub fn wait_pid(&self, pid: ProcessId) {
        let mut q = self.queue.lock();
        if !q.contains(&pid) {
            q.push(pid);
        }
    }

    /// Enqueues the currently running process and blocks it.
    pub fn wait_current(&self) {
        let current_pid = {
            let pm = PROCESS_MANAGER.lock();
            pm.current().map(|p| p.pid).unwrap_or(ProcessId::KERNEL)
        };
        self.wait_pid(current_pid);
        crate::scheduler::block_current();
    }

    /// Wakes the oldest waiting process, transitioning it to Ready.
    pub fn wake_one(&self) -> Option<ProcessId> {
        let pid = {
            let mut q = self.queue.lock();
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        };

        if let Some(woken_pid) = pid {
            crate::scheduler::wake_task(woken_pid);
            Some(woken_pid)
        } else {
            None
        }
    }

    /// Wakes all waiting processes in the queue.
    pub fn wake_all(&self) -> usize {
        let pids: Vec<ProcessId> = {
            let mut q = self.queue.lock();
            q.drain(..).collect()
        };

        let count = pids.len();
        for pid in pids {
            crate::scheduler::wake_task(pid);
        }
        count
    }

    /// Returns the number of tasks currently waiting.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    /// Returns true if no tasks are currently waiting.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }
}
