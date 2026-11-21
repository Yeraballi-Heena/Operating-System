//! Interrupt-Safe Input Event Ring Buffer
//!
//! Provides a lock-safe bounded circular FIFO queue for buffering `InputEvent`s
//! between hardware interrupt handlers and consumer tasks.

use crate::input::events::InputEvent;

pub const INPUT_QUEUE_CAPACITY: usize = 256;

/// Fixed-capacity circular queue for `InputEvent`s.
pub struct InputEventQueue {
    buffer: [Option<InputEvent>; INPUT_QUEUE_CAPACITY],
    read_idx: usize,
    write_idx: usize,
    count: usize,
    overflow_count: u64,
}

impl InputEventQueue {
    pub const fn new() -> Self {
        Self {
            buffer: [None; INPUT_QUEUE_CAPACITY],
            read_idx: 0,
            write_idx: 0,
            count: 0,
            overflow_count: 0,
        }
    }

    /// Pushes an event into the queue.
    /// If full, overwrites the oldest unread event and increments overflow counter.
    pub fn push(&mut self, event: InputEvent) {
        if self.count == INPUT_QUEUE_CAPACITY {
            // Buffer full: drop oldest event to avoid input lag
            self.read_idx = (self.read_idx + 1) % INPUT_QUEUE_CAPACITY;
            self.count -= 1;
            self.overflow_count += 1;
        }

        self.buffer[self.write_idx] = Some(event);
        self.write_idx = (self.write_idx + 1) % INPUT_QUEUE_CAPACITY;
        self.count += 1;
    }

    /// Dequeues the oldest available input event.
    pub fn pop(&mut self) -> Option<InputEvent> {
        if self.count == 0 {
            None
        } else {
            let event = self.buffer[self.read_idx].take();
            self.read_idx = (self.read_idx + 1) % INPUT_QUEUE_CAPACITY;
            self.count -= 1;
            event
        }
    }

    /// Peeks at the next available event without dequeuing it.
    pub fn peek(&self) -> Option<InputEvent> {
        if self.count == 0 {
            None
        } else {
            self.buffer[self.read_idx]
        }
    }

    /// Returns the number of events currently queued.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the queue contains no events.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns total events dropped due to buffer overflow.
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }

    /// Empties all events from the queue.
    pub fn clear(&mut self) {
        for slot in self.buffer.iter_mut() {
            *slot = None;
        }
        self.read_idx = 0;
        self.write_idx = 0;
        self.count = 0;
    }
}
