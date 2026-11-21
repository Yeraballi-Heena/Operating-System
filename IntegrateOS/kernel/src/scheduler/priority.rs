//! Scheduler Task Priority Levels
//!
//! Provides priority bands, conversion from numeric priorities,
//! and per-priority timeslice calculation.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    Realtime = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Idle = 4,
}

impl Priority {
    /// Maps a raw numeric priority (0..=255) into a scheduler priority band.
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Realtime,
            1..=10 => Self::High,
            11..=50 => Self::Normal,
            51..=254 => Self::Low,
            255 => Self::Idle,
        }
    }

    /// Returns the default timeslice (in 100 Hz timer ticks) for this priority level.
    pub const fn default_timeslice(self) -> u32 {
        match self {
            Self::Realtime => 20, // 200 ms
            Self::High => 10,     // 100 ms
            Self::Normal => 5,    // 50 ms
            Self::Low => 2,       // 20 ms
            Self::Idle => 1,      // 10 ms
        }
    }
}
