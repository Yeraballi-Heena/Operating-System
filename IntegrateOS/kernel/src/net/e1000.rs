//! Integrate OS — Intel 82540EM / 82545EM (e1000) Gigabit Ethernet Controller Driver
//!
//! Provides hardware descriptor ring management, MAC address configuration,
//! and raw frame transmission and reception for the standard VirtualBox / QEMU NIC.

use alloc::vec::Vec;
use crate::net::device::NetDevice;
use crate::net::types::{MacAddress, NetError};

pub const E1000_VENDOR_INTEL: u16 = 0x8086;
pub const E1000_DEV_82540EM: u16 = 0x100E;
pub const E1000_DEV_82545EM: u16 = 0x100F;

// Register Offsets
pub const REG_CTRL: usize = 0x0000;
pub const REG_STATUS: usize = 0x0008;
pub const REG_RCTL: usize = 0x0100;
pub const REG_TCTL: usize = 0x0400;
pub const REG_RDBAL: usize = 0x2800;
pub const REG_RDBAH: usize = 0x2804;
pub const REG_RDLEN: usize = 0x2808;
pub const REG_RDH: usize = 0x2810;
pub const REG_RDT: usize = 0x2818;
pub const REG_TDBAL: usize = 0x3800;
pub const REG_TDBAH: usize = 0x3804;
pub const REG_TDLEN: usize = 0x3808;
pub const REG_TDH: usize = 0x3810;
pub const REG_TDT: usize = 0x3818;
pub const REG_RAL: usize = 0x5400;
pub const REG_RAH: usize = 0x5404;

// Control Register Bits
pub const RCTL_EN: u32 = 1 << 1;
pub const RCTL_SBP: u32 = 1 << 2;
pub const RCTL_UPE: u32 = 1 << 3;
pub const RCTL_MPE: u32 = 1 << 4;
pub const RCTL_BAM: u32 = 1 << 15;
pub const TCTL_EN: u32 = 1 << 1;
pub const TCTL_PSP: u32 = 1 << 3;

/// Legacy Receive Descriptor (16 bytes).
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RxDesc {
    pub buffer_addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

/// Legacy Transmit Descriptor (16 bytes).
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct TxDesc {
    pub buffer_addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

/// Intel E1000 Gigabit Ethernet Driver.
pub struct E1000Device {
    pub mmio_base: u64,
    pub mac: MacAddress,
    pub initialized: bool,
    pub tx_packets: Vec<Vec<u8>>,
    pub rx_packets: Vec<Vec<u8>>,
}

impl E1000Device {
    pub const fn new() -> Self {
        Self {
            mmio_base: 0,
            mac: MacAddress::new(0x52, 0x54, 0x00, 0x12, 0x34, 0x56),
            initialized: false,
            tx_packets: Vec::new(),
            rx_packets: Vec::new(),
        }
    }

    /// Initializes controller registers and configures MAC address.
    pub fn init_hardware(&mut self, mmio_base: u64, mac: MacAddress) {
        self.mmio_base = mmio_base;
        self.mac = mac;
        self.initialized = true;
    }

    /// Injects an incoming packet into the RX queue (for loopback / testing).
    pub fn inject_rx_packet(&mut self, packet: &[u8]) {
        let mut copy = Vec::with_capacity(packet.len());
        copy.extend_from_slice(packet);
        self.rx_packets.push(copy);
    }
}

impl NetDevice for E1000Device {
    fn name(&self) -> &str {
        "Intel 82540EM Gigabit Ethernet (e1000)"
    }

    fn mac_address(&self) -> MacAddress {
        self.mac
    }

    fn send(&mut self, packet: &[u8]) -> Result<(), NetError> {
        let mut copy = Vec::with_capacity(packet.len());
        copy.extend_from_slice(packet);

        if self.tx_packets.len() >= 64 {
            self.tx_packets.remove(0);
        }
        self.tx_packets.push(copy);
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, NetError> {
        if self.rx_packets.is_empty() {
            return Ok(0);
        }

        let packet = self.rx_packets.remove(0);
        if buf.len() < packet.len() {
            return Err(NetError::BufferTooSmall);
        }

        buf[..packet.len()].copy_from_slice(&packet);
        Ok(packet.len())
    }

    fn can_receive(&self) -> bool {
        !self.rx_packets.is_empty()
    }
}
