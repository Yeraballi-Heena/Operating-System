//! Integrate OS — Address Resolution Protocol (ARP, RFC 826)
//!
//! Translates 32-bit IPv4 addresses to 48-bit Ethernet MAC addresses and maintains
//! the dynamic host ARP cache table.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::types::{Ipv4Address, MacAddress, NetError};

pub const ARP_PACKET_LEN: usize = 28;
pub const ARP_HW_ETHERNET: u16 = 1;
pub const ARP_PROTO_IPV4: u16 = 0x0800;

/// ARP Operation Code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArpOpcode {
    Request,
    Reply,
    Unknown(u16),
}

impl ArpOpcode {
    pub const fn from_u16(val: u16) -> Self {
        match val {
            1 => Self::Request,
            2 => Self::Reply,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_u16(self) -> u16 {
        match self {
            Self::Request => 1,
            Self::Reply => 2,
            Self::Unknown(val) => val,
        }
    }
}

/// 28-byte RFC 826 ARP Packet structure for Ethernet / IPv4.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArpPacket {
    pub opcode: ArpOpcode,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Address,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Address,
}

impl ArpPacket {
    pub fn new_request(sender_mac: MacAddress, sender_ip: Ipv4Address, target_ip: Ipv4Address) -> Self {
        Self {
            opcode: ArpOpcode::Request,
            sender_mac,
            sender_ip,
            target_mac: MacAddress::ZERO,
            target_ip,
        }
    }

    pub fn new_reply(
        sender_mac: MacAddress,
        sender_ip: Ipv4Address,
        target_mac: MacAddress,
        target_ip: Ipv4Address,
    ) -> Self {
        Self {
            opcode: ArpOpcode::Reply,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }

    /// Parses an ARP packet from raw bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, NetError> {
        if bytes.len() < ARP_PACKET_LEN {
            return Err(NetError::PacketMalformed);
        }

        let hw_type = u16::from_be_bytes([bytes[0], bytes[1]]);
        let proto_type = u16::from_be_bytes([bytes[2], bytes[3]]);
        let hw_size = bytes[4];
        let proto_size = bytes[5];
        let opcode_raw = u16::from_be_bytes([bytes[6], bytes[7]]);

        if hw_type != ARP_HW_ETHERNET || proto_type != ARP_PROTO_IPV4 || hw_size != 6 || proto_size != 4 {
            return Err(NetError::PacketMalformed);
        }

        let mut smac = [0u8; 6];
        smac.copy_from_slice(&bytes[8..14]);

        let mut sip = [0u8; 4];
        sip.copy_from_slice(&bytes[14..18]);

        let mut tmac = [0u8; 6];
        tmac.copy_from_slice(&bytes[18..24]);

        let mut tip = [0u8; 4];
        tip.copy_from_slice(&bytes[24..28]);

        Ok(Self {
            opcode: ArpOpcode::from_u16(opcode_raw),
            sender_mac: MacAddress(smac),
            sender_ip: Ipv4Address(sip),
            target_mac: MacAddress(tmac),
            target_ip: Ipv4Address(tip),
        })
    }

    /// Serializes the ARP packet into a 28-byte buffer.
    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        if buf.len() < ARP_PACKET_LEN {
            return Err(NetError::BufferTooSmall);
        }

        buf[0..2].copy_from_slice(&ARP_HW_ETHERNET.to_be_bytes());
        buf[2..4].copy_from_slice(&ARP_PROTO_IPV4.to_be_bytes());
        buf[4] = 6;
        buf[5] = 4;
        buf[6..8].copy_from_slice(&self.opcode.to_u16().to_be_bytes());
        buf[8..14].copy_from_slice(self.sender_mac.as_bytes());
        buf[14..18].copy_from_slice(self.sender_ip.as_bytes());
        buf[18..24].copy_from_slice(self.target_mac.as_bytes());
        buf[24..28].copy_from_slice(self.target_ip.as_bytes());

        Ok(ARP_PACKET_LEN)
    }

    /// Converts the ARP packet into a raw byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; ARP_PACKET_LEN];
        let _ = self.serialize(&mut buf);
        buf
    }
}

/// An entry in the ARP cache table.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArpEntry {
    pub ip: Ipv4Address,
    pub mac: MacAddress,
    pub updated_tick: u64,
}

/// Dynamic ARP Cache Table.
pub struct ArpTable {
    pub entries: Vec<ArpEntry>,
}

impl ArpTable {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Records or updates an IP-to-MAC mapping in the ARP cache.
    pub fn insert(&mut self, ip: Ipv4Address, mac: MacAddress, tick: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.ip == ip) {
            entry.mac = mac;
            entry.updated_tick = tick;
            return;
        }

        // Limit cache size to 64 active entries
        if self.entries.len() >= 64 {
            self.entries.remove(0);
        }

        self.entries.push(ArpEntry {
            ip,
            mac,
            updated_tick: tick,
        });
    }

    /// Resolves an IP address to its known MAC address.
    pub fn lookup(&self, ip: Ipv4Address) -> Option<MacAddress> {
        // Special case: Loopback or Broadcast
        if ip.is_loopback() {
            return Some(MacAddress::new(0x02, 0x00, 0x00, 0x00, 0x00, 0x01));
        }
        if ip.is_broadcast() {
            return Some(MacAddress::BROADCAST);
        }

        self.entries.iter().find(|e| e.ip == ip).map(|e| e.mac)
    }

    /// Clears all entries from the ARP cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
