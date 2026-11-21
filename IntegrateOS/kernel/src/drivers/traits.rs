//! Hardware Support Classification Matrix
//!
//! Mandatory Engineering Discipline:
//! The system explicitly classifies hardware capability without faking completion:
//! - `Supported`: Fully implemented, operational, and tested in VirtualBox/hardware.
//! - `Partial`: Baseline operational; advanced/vendor-specific features omitted.
//! - `Foundation`: Architectural interfaces, driver structures, and discovery defined.
//! - `Unsupported`: Device or chipset is recognized or unhandled, but not driven.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareSupportStatus {
    Supported,
    Partial,
    Foundation,
    Unsupported,
}

impl HardwareSupportStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Supported => "SUPPORTED",
            Self::Partial => "PARTIAL",
            Self::Foundation => "FOUNDATION",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}
