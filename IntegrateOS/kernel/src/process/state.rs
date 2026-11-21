//! Process Lifecycle States
//!
//! Models the architectural states of an operating system process.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Runnable and queued for CPU scheduling.
    Ready,
    /// Actively executing on a CPU core.
    Running,
    /// Blocked awaiting an I/O event, lock, or hardware interrupt.
    Blocked,
    /// Sleeping until a specific system timer tick target is reached.
    Sleeping(u64),
    /// Terminated with an integer exit code (awaiting parent reap / cleanup).
    Terminated(i32),
}

impl ProcessState {
    /// Returns a human-readable display string for this state.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Running => "RUNNING",
            Self::Blocked => "BLOCKED",
            Self::Sleeping(_) => "SLEEPING",
            Self::Terminated(_) => "TERMINATED",
        }
    }

    /// Returns true if the process is eligible for CPU dispatch.
    #[inline(always)]
    pub const fn is_runnable(&self) -> bool {
        matches!(self, Self::Ready | Self::Running)
    }

    /// Returns true if the process has halted and exited.
    #[inline(always)]
    pub const fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated(_))
    }
}
