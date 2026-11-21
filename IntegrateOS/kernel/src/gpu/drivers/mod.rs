//! Integrate OS — GPU Hardware Drivers
//!
//! Exposes physical PCI graphics drivers and universal software fallback implementations.

pub mod qemu;
pub mod software;
pub mod vbox;
pub mod vmware;

pub use qemu::QemuStdVgaDriver;
pub use software::SoftwareGpuDriver;
pub use vbox::VboxGpuDriver;
pub use vmware::VmwareSvgaDriver;
