//! Integrate OS — Software Timer Service
//!
//! Provides one-shot and periodic timer registration, deadline checking,
//! interval rescheduling, and event polling.

use alloc::vec::Vec;

pub type TimerId = u32;

/// Timer operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// Fires once upon deadline expiration and ceases.
    OneShot,
    /// Automatically resets deadline by interval upon expiration.
    Periodic,
}

/// Software timer registration entry.
#[derive(Debug, Clone)]
pub struct TimerEntry {
    pub id: TimerId,
    pub mode: TimerMode,
    pub deadline_ms: u64,
    pub interval_ms: u64,
    pub expired: bool,
    pub active: bool,
}

/// Software Timer Service.
pub struct TimerService {
    timers: Vec<TimerEntry>,
    next_timer_id: TimerId,
}

impl TimerService {
    pub const fn new() -> Self {
        Self {
            timers: Vec::new(),
            next_timer_id: 1,
        }
    }

    /// Registers a new software timer.
    pub fn create_timer(&mut self, mode: TimerMode, duration_ms: u64, current_time_ms: u64) -> TimerId {
        let id = self.next_timer_id;
        self.next_timer_id = self.next_timer_id.wrapping_add(1);

        let entry = TimerEntry {
            id,
            mode,
            deadline_ms: current_time_ms.saturating_add(duration_ms),
            interval_ms: duration_ms,
            expired: false,
            active: true,
        };

        self.timers.push(entry);
        id
    }

    /// Cancels an active timer by ID.
    pub fn cancel(&mut self, id: TimerId) -> bool {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            self.timers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Checks if a timer has expired.
    pub fn is_expired(&self, id: TimerId) -> bool {
        self.timers
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.expired)
            .unwrap_or(false)
    }

    /// Resets an expired timer flag.
    pub fn clear_expired(&mut self, id: TimerId) {
        if let Some(t) = self.timers.iter_mut().find(|t| t.id == id) {
            t.expired = false;
        }
    }

    /// Evaluates all active timers against current monotonic uptime.
    /// Returns a list of expired timer IDs.
    pub fn poll_expired(&mut self, current_time_ms: u64) -> Vec<TimerId> {
        let mut expired_ids = Vec::new();

        for timer in self.timers.iter_mut() {
            if !timer.active {
                continue;
            }

            if current_time_ms >= timer.deadline_ms {
                timer.expired = true;
                expired_ids.push(timer.id);

                match timer.mode {
                    TimerMode::OneShot => {
                        timer.active = false;
                    }
                    TimerMode::Periodic => {
                        timer.deadline_ms = current_time_ms.saturating_add(timer.interval_ms);
                    }
                }
            }
        }

        // Clean up inactive one-shot timers that have expired
        self.timers.retain(|t| t.active || t.expired);

        expired_ids
    }

    /// Returns the number of currently active timers.
    pub fn active_count(&self) -> usize {
        self.timers.iter().filter(|t| t.active).count()
    }
}
