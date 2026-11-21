//! Integrate OS — Network Types & Addressing
//!
//! Provides fundamental data structures for link-layer (MAC), network-layer (IPv4),
//! and transport-layer (Port/SocketAddr) networking.

use alloc::string::String;
use core::fmt;

/// 48-bit IEEE 802.3 MAC (Media Access Control) Hardware Address.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xFF; 6]);
    pub const ZERO: Self = Self([0; 6]);

    pub const fn new(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8) -> Self {
        Self([b0, b1, b2, b3, b4, b5])
    }

    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    pub const fn is_broadcast(&self) -> bool {
        self.0[0] == 0xFF
            && self.0[1] == 0xFF
            && self.0[2] == 0xFF
            && self.0[3] == 0xFF
            && self.0[4] == 0xFF
            && self.0[5] == 0xFF
    }

    pub const fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0 && !self.is_broadcast()
    }

    pub const fn is_unicast(&self) -> bool {
        (self.0[0] & 0x01) == 0
    }

    /// Parses a MAC address from a hex string (e.g. "52:54:00:12:34:56").
    pub fn parse_str(s: &str) -> Option<Self> {
        let mut bytes = [0u8; 6];
        let mut idx = 0;

        for part in s.split(':') {
            if idx >= 6 || part.len() != 2 {
                return None;
            }
            let b = u8::from_str_radix(part, 16).ok()?;
            bytes[idx] = b;
            idx += 1;
        }

        if idx == 6 {
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Formats the MAC address into a standard hex string.
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(17);
        for (i, &b) in self.0.iter().enumerate() {
            if i > 0 {
                s.push(':');
            }
            let hex = [b"0123456789ABCDEF"[(b >> 4) as usize] as char,
                       b"0123456789ABCDEF"[(b & 0x0F) as usize] as char];
            s.push(hex[0]);
            s.push(hex[1]);
        }
        s
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// 32-bit IPv4 (Internet Protocol version 4) Address.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    pub const LOCALHOST: Self = Self([127, 0, 0, 1]);
    pub const BROADCAST: Self = Self([255, 255, 255, 255]);
    pub const UNSPECIFIED: Self = Self([0, 0, 0, 0]);

    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    pub const fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    pub const fn from_u32(val: u32) -> Self {
        Self(val.to_be_bytes())
    }

    pub const fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    pub const fn is_broadcast(&self) -> bool {
        self.0[0] == 255 && self.0[1] == 255 && self.0[2] == 255 && self.0[3] == 255
    }

    pub const fn is_unspecified(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    /// Checks if this address belongs to the same subnet as `other` given a `subnet_mask`.
    pub fn is_in_subnet(&self, other: Self, mask: Self) -> bool {
        let a = self.to_u32();
        let b = other.to_u32();
        let m = mask.to_u32();
        (a & m) == (b & m)
    }

    /// Parses an IPv4 address from dotted decimal string (e.g. "192.168.1.1").
    pub fn parse_str(s: &str) -> Option<Self> {
        let mut bytes = [0u8; 4];
        let mut idx = 0;

        for part in s.split('.') {
            if idx >= 4 {
                return None;
            }
            let b = part.parse::<u8>().ok()?;
            bytes[idx] = b;
            idx += 1;
        }

        if idx == 4 {
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Formats the IPv4 address into a dotted decimal string.
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(15);
        for (i, &b) in self.0.iter().enumerate() {
            if i > 0 {
                s.push('.');
            }
            format_u8_dec(&mut s, b);
        }
        s
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl fmt::Debug for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// Ethernet II Frame EtherType identifiers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EtherType {
    Ipv4,
    Arp,
    Ipv6,
    Unknown(u16),
}

impl EtherType {
    pub const fn from_u16(val: u16) -> Self {
        match val {
            0x0800 => Self::Ipv4,
            0x0806 => Self::Arp,
            0x86DD => Self::Ipv6,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_u16(self) -> u16 {
        match self {
            Self::Ipv4 => 0x0800,
            Self::Arp => 0x0806,
            Self::Ipv6 => 0x86DD,
            Self::Unknown(val) => val,
        }
    }
}

/// IPv4 Transport Protocol Numbers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IpProtocol {
    Icmp,
    Tcp,
    Udp,
    Unknown(u8),
}

impl IpProtocol {
    pub const fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Icmp,
            6 => Self::Tcp,
            17 => Self::Udp,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Icmp => 1,
            Self::Tcp => 6,
            Self::Udp => 17,
            Self::Unknown(val) => val,
        }
    }
}

/// IPv4 Network Socket Endpoint Address (IP + Port).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SocketAddr {
    pub ip: Ipv4Address,
    pub port: u16,
}

impl SocketAddr {
    pub const fn new(ip: Ipv4Address, port: u16) -> Self {
        Self { ip, port }
    }
}

/// Common Network Stack Errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    BufferTooSmall,
    InvalidChecksum,
    PacketMalformed,
    ArpNotFound,
    PortInUse,
    ConnectionRefused,
    Timeout,
    DeviceError(&'static str),
    SocketClosed,
    NotConnected,
}

fn format_u8_dec(s: &mut String, val: u8) {
    if val == 0 {
        s.push('0');
        return;
    }
    let mut digits = [0u8; 3];
    let mut len = 0;
    let mut temp = val;
    while temp > 0 {
        digits[len] = b'0' + (temp % 10);
        temp /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        s.push(digits[i] as char);
    }
}
