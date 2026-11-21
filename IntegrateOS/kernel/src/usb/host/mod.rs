//! Integrate OS — USB Host Controllers
//!
//! Exposes physical and virtual USB Host Controller implementations.

pub mod ehci;
pub mod uhci;
pub mod virtual_host;
pub mod xhci;

pub use ehci::EhciController;
pub use uhci::UhciController;
pub use virtual_host::VirtualUsbHostController;
pub use xhci::XhciController;
