//! Integrate OS — Hardware Architecture Layer
//!
//! Isolates CPU-specific instructions, descriptor tables, control registers,
//! privilege rings, and context state from generic kernel subsystems.

pub mod x86_64;
