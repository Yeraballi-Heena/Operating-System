//! Integrate OS — Network Interface Configuration
//!
//! Provides static IP, subnet mask, gateway, and DNS configuration structures.

use crate::net::types::Ipv4Address;

/// IPv4 Network Interface Configuration parameters.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InterfaceConfig {
    pub ip: Ipv4Address,
    pub subnet_mask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub dns: Ipv4Address,
}

impl InterfaceConfig {
    pub const fn new(
        ip: Ipv4Address,
        subnet_mask: Ipv4Address,
        gateway: Ipv4Address,
        dns: Ipv4Address,
    ) -> Self {
        Self {
            ip,
            subnet_mask,
            gateway,
            dns,
        }
    }

    /// Default VirtualBox NAT configuration: IP 10.0.2.15, Gateway 10.0.2.2, DNS 10.0.2.3.
    pub const fn virtualbox_default() -> Self {
        Self {
            ip: Ipv4Address::new(10, 0, 2, 15),
            subnet_mask: Ipv4Address::new(255, 255, 255, 0),
            gateway: Ipv4Address::new(10, 0, 2, 2),
            dns: Ipv4Address::new(10, 0, 2, 3),
        }
    }

    /// Localhost configuration: 127.0.0.1.
    pub const fn loopback() -> Self {
        Self {
            ip: Ipv4Address::LOCALHOST,
            subnet_mask: Ipv4Address::new(255, 0, 0, 0),
            gateway: Ipv4Address::UNSPECIFIED,
            dns: Ipv4Address::UNSPECIFIED,
        }
    }
}
