//! Unified Hardware Driver Framework
//!
//! Provides trait-based device abstractions, hardware classification matrix,
//! and a dynamic device registry.

pub mod device;
pub mod hwinfo;
pub mod manager;
pub mod traits;
pub mod wrappers;
pub mod laptop;
pub mod laptop_tests;
