//! Integrate OS — Network Subsystem Architecture
//!
//! Provides an end-to-end bare-metal networking stack:
//! - Layer 2: Ethernet II framing (`ethernet.rs`)
//! - Layer 3: ARP resolution (`arp.rs`), IPv4 packet routing & fragmentation (`ipv4.rs`), ICMP diagnostics (`icmp.rs`)
//! - Layer 4: Connectionless UDP (`udp.rs`), Reliable Stream TCP (`tcp.rs`)
//! - Drivers: Software Loopback (`device.rs`), Intel 82540EM Gigabit Ethernet (`e1000.rs`)
//! - Socket API: Unified socket descriptors and port multiplexing (`socket.rs`)
//! - Verification: 5-stage test suite (`tests.rs`)

pub mod arp;
pub mod checksum;
pub mod device;
pub mod dhcp;
pub mod dns;
pub mod e1000;
pub mod ethernet;
pub mod icmp;
pub mod interface;
pub mod ipv4;
pub mod phase35_tests;
pub mod socket;
pub mod stack;
pub mod tcp;
pub mod tests;
pub mod types;
pub mod udp;
pub mod wifi;
pub mod wifi_real_tests;

pub use dhcp::{DhcpClient, DhcpLease, DhcpMessageType};
pub use dns::resolve as dns_resolve;
pub use interface::InterfaceConfig;
pub use socket::{bind, close, recv_from, send_to, socket, SocketHandle, SocketType};
pub use stack::{NetworkStack, NETWORK_STACK};
pub use types::*;
pub use wifi::{WifiAccessPoint, WifiAuth, WifiState, WIFI_MANAGER};

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static NETWORK_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the network stack has been successfully initialized.
pub fn is_network_initialized() -> bool {
    NETWORK_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the central network stack and registers network device interfaces.
pub fn init() {
    wifi::init();
    NETWORK_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[net] Network Subsystem initialized (Ethernet LAN, 802.11 WiFi, ARP, IPv4, ICMP, UDP, TCP, Sockets)");
}
