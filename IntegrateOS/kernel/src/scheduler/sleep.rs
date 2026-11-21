//! Scheduler Sleep Queue & Timed Task Suspension
//!
//! Manages processes suspended until a target monotonic timer tick,
//! automatically waking them when the timer advances past their expiry.

use alloc::vec::Vec;

use crate::process::pid::ProcessId;

#[derive(Debug, Clone, Copy)]
pub struct SleepEntry {
    pub pid: ProcessId,
    pub wake_tick: u64,
}

pub struct SleepQueue {
    entries: Vec<SleepEntry>,
}

impl SleepQueue {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Suspends a process until `wake_tick`.
    pub fn add(&mut self, pid: ProcessId, wake_tick: u64) {
        self.entries.retain(|e| e.pid != pid);
        self.entries.push(SleepEntry { pid, wake_tick });
    }

    /// Wakes all processes whose `wake_tick` is less than or equal to `current_tick`.
    pub fn wake_expired(&mut self, current_tick: u64) -> Vec<ProcessId> {
        let mut woken = Vec::new();
        self.entries.retain(|e| {
            if current_tick >= e.wake_tick {
                woken.push(e.pid);
                false
            } else {
                true
            }
        });
        woken
    }

    /// Removes a process from the sleep queue if present.
    pub fn remove(&mut self, pid: ProcessId) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.pid != pid);
        self.entries.len() < initial_len
    }

    /// Returns the number of currently sleeping processes.
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no processes are currently sleeping.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears the sleep queue.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
