//! Multi-Level Priority Run Queue Architecture
//!
//! Provides prioritized FIFO task scheduling across Realtime, High,
//! Normal, Low, and Idle priority bands.

use alloc::collections::VecDeque;

use crate::process::pid::ProcessId;
use crate::scheduler::priority::Priority;

pub const PRIORITY_LEVELS: usize = 5;

pub struct RunQueue {
    queues: [VecDeque<ProcessId>; PRIORITY_LEVELS],
    count: usize,
}

impl RunQueue {
    /// Creates an empty multi-level run queue.
    pub const fn new() -> Self {
        Self {
            queues: [
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
            ],
            count: 0,
        }
    }

    /// Enqueues a process into the bucket matching its priority band.
    pub fn enqueue(&mut self, pid: ProcessId, priority: Priority) {
        let idx = priority as usize;
        if idx < PRIORITY_LEVELS && !self.contains(pid) {
            self.queues[idx].push_back(pid);
            self.count += 1;
        }
    }

    /// Dequeues the next runnable process from the highest non-empty priority bucket.
    pub fn dequeue(&mut self) -> Option<ProcessId> {
        for q in self.queues.iter_mut() {
            if let Some(pid) = q.pop_front() {
                self.count -= 1;
                return Some(pid);
            }
        }
        None
    }

    /// Removes a specific process from all priority queues.
    pub fn remove(&mut self, pid: ProcessId) -> bool {
        for q in self.queues.iter_mut() {
            if let Some(pos) = q.iter().position(|&p| p == pid) {
                q.remove(pos);
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Returns `true` if `pid` is currently enqueued.
    pub fn contains(&self, pid: ProcessId) -> bool {
        for q in self.queues.iter() {
            if q.contains(&pid) {
                return true;
            }
        }
        false
    }

    /// Returns `true` if no processes are currently ready.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the total number of ready processes across all buckets.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Clears all ready queues.
    pub fn clear(&mut self) {
        for q in self.queues.iter_mut() {
            q.clear();
        }
        self.count = 0;
    }
}
