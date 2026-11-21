//! Integrate OS — Process Subsystem Architecture
//!
//! Provides:
//! - Process Control Block (`ProcessControlBlock`)
//! - Monotonic unique PID allocation (`ProcessId`)
//! - Process lifecycle state transitions (`ProcessState`)
//! - Hardware context saving and switching (`TaskContext`, `switch_context_asm`)
//! - Central process table registry (`PROCESS_MANAGER`)
//! - Process verification suite (`tests::run_process_tests`)

pub mod context;
pub mod manager;
pub mod pcb;
pub mod pid;
pub mod state;
pub mod switch;
pub mod tests;

pub use context::TaskContext;
pub use manager::{ProcessManager, PROCESS_MANAGER};
pub use pcb::ProcessControlBlock;
pub use pid::ProcessId;
pub use state::ProcessState;

/// Initializes the central process manager and root bootstrap process (PID 0).
pub fn init() -> Result<(), &'static str> {
    PROCESS_MANAGER.lock().init()
}
