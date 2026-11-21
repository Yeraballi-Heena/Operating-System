//! Thread Rendezvous Barrier
//!
//! Enables a fixed number of tasks to synchronize at a shared execution checkpoint.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Result returned from `Barrier::wait`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarrierWaitResult {
    is_leader: bool,
}

impl BarrierWaitResult {
    /// Returns true if this thread was the leader (the last to reach the barrier).
    #[inline(always)]
    pub fn is_leader(&self) -> bool {
        self.is_leader
    }
}

/// A synchronization barrier enabling `N` threads to rendezvous.
pub struct Barrier {
    num_threads: usize,
    count: AtomicUsize,
    generation: AtomicUsize,
}

impl Barrier {
    /// Creates a new `Barrier` for `num_threads` participants.
    pub const fn new(num_threads: usize) -> Self {
        Self {
            num_threads,
            count: AtomicUsize::new(num_threads),
            generation: AtomicUsize::new(0),
        }
    }

    /// Blocks until all `num_threads` participants have reached the barrier.
    /// Exactly one participant will receive `BarrierWaitResult::is_leader() == true`.
    pub fn wait(&self) -> BarrierWaitResult {
        let local_gen = self.generation.load(Ordering::Acquire);
        let prev = self.count.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            // Last thread to arrive: reset count and advance generation
            self.count.store(self.num_threads, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            BarrierWaitResult { is_leader: true }
        } else {
            // Wait for leader to advance generation
            while self.generation.load(Ordering::Acquire) == local_gen {
                crate::scheduler::yield_now();
            }
            BarrierWaitResult { is_leader: false }
        }
    }
}
