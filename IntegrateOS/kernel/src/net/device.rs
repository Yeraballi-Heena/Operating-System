//! Integrate OS — Network Device Interface & Loopback Device
//!
//! Provides the core abstraction for physical and virtual network interface cards,
//! along with a high-performance software loopback adapter.

use alloc::vec::Vec;
use crate::net::types::{MacAddress, NetError};

/// Polymorphic interface for all physical, virtual, and loopback network controllers.
pub trait NetDevice: Send + Sync {
    /// Human-readable network interface device name.
    fn name(&self) -> &str;

    /// Returns the hardware MAC address of the interface.
    fn mac_address(&self) -> MacAddress;

    /// Transmits a raw Ethernet frame out over the network interface.
    fn send(&mut self, packet: &[u8]) -> Result<(), NetError>;

    /// Receives a raw Ethernet frame into `buf`. Returns bytes read.
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, NetError>;

    /// Returns `true` if an incoming packet is queued and ready for processing.
    fn can_receive(&self) -> bool;
}

/// In-memory Software Loopback Network Adapter (`lo0`).
pub struct LoopbackDevice {
    mac: MacAddress,
    queue: Vec<Vec<u8>>,
}

impl LoopbackDevice {
    pub const fn new() -> Self {
        Self {
            mac: MacAddress::new(0x02, 0x00, 0x00, 0x00, 0x00, 0x01),
            queue: Vec::new(),
        }
    }
}

impl NetDevice for LoopbackDevice {
    fn name(&self) -> &str {
        "Loopback Interface (lo0)"
    }

    fn mac_address(&self) -> MacAddress {
        self.mac
    }

    fn send(&mut self, packet: &[u8]) -> Result<(), NetError> {
        if self.queue.len() >= 64 {
            return Err(NetError::BufferTooSmall);
        }
        let mut copy = Vec::with_capacity(packet.len());
        copy.extend_from_slice(packet);
        self.queue.push(copy);
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, NetError> {
        if self.queue.is_empty() {
            return Ok(0);
        }

        let packet = self.queue.remove(0);
        if buf.len() < packet.len() {
            return Err(NetError::BufferTooSmall);
        }

        buf[..packet.len()].copy_from_slice(&packet);
        Ok(packet.len())
    }

    fn can_receive(&self) -> bool {
        !self.queue.is_empty()
    }
}
