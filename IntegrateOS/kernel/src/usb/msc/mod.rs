//! Integrate OS — USB Mass Storage Subsystem
//!
//! Provides Bulk-Only Transport (BOT) and SCSI Transparent Command Set drivers.

pub mod bot;
pub mod driver;
pub mod scsi;

pub use bot::{CommandBlockWrapper, CommandStatusWrapper};
pub use driver::UsbStorageDevice;
pub use scsi::{ScsiCapacityData, ScsiInquiryData};
