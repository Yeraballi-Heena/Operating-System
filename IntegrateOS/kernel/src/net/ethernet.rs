//! Integrate OS — Ethernet II Framing Layer
//!
//! Provides IEEE 802.3 Ethernet frame parsing, serialization, and payload demultiplexing.

use alloc::vec::Vec;
use crate::net::types::{EtherType, MacAddress, NetError};

pub const ETHERNET_HEADER_LEN: usize = 14;
pub const ETHERNET_MIN_FRAME: usize = 60;
pub const ETHERNET_MAX_FRAME: usize = 1514;
pub const ETHERNET_MTU: usize = 1500;

/// 14-byte Ethernet II Frame Header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EthernetHeader {
    pub dst_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: EtherType,
}

impl EthernetHeader {
    pub const fn new(dst_mac: MacAddress, src_mac: MacAddress, ethertype: EtherType) -> Self {
        Self {
            dst_mac,
            src_mac,
            ethertype,
        }
    }

    /// Parses an Ethernet II header from raw packet bytes.
    pub fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), NetError> {
        if bytes.len() < ETHERNET_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        let mut dst = [0u8; 6];
        dst.copy_from_slice(&bytes[0..6]);

        let mut src = [0u8; 6];
        src.copy_from_slice(&bytes[6..12]);

        let ethertype_raw = u16::from_be_bytes([bytes[12], bytes[13]]);
        let ethertype = EtherType::from_u16(ethertype_raw);

        let header = Self {
            dst_mac: MacAddress(dst),
            src_mac: MacAddress(src),
            ethertype,
        };

        Ok((header, &bytes[ETHERNET_HEADER_LEN..]))
    }

    /// Serializes the header into a byte buffer (requires at least 14 bytes).
    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        if buf.len() < ETHERNET_HEADER_LEN {
            return Err(NetError::BufferTooSmall);
        }

        buf[0..6].copy_from_slice(self.dst_mac.as_bytes());
        buf[6..12].copy_from_slice(self.src_mac.as_bytes());
        let eth_bytes = self.ethertype.to_u16().to_be_bytes();
        buf[12] = eth_bytes[0];
        buf[13] = eth_bytes[1];

        Ok(ETHERNET_HEADER_LEN)
    }
}

/// Constructs a complete Ethernet II frame from header fields and payload.
pub fn build_ethernet_frame(
    dst_mac: MacAddress,
    src_mac: MacAddress,
    ethertype: EtherType,
    payload: &[u8],
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(ETHERNET_HEADER_LEN + payload.len().max(46));
    frame.extend_from_slice(dst_mac.as_bytes());
    frame.extend_from_slice(src_mac.as_bytes());
    frame.extend_from_slice(&ethertype.to_u16().to_be_bytes());
    frame.extend_from_slice(payload);

    // Zero-pad to minimum payload size (46 bytes -> 60 byte total frame)
    while frame.len() < ETHERNET_MIN_FRAME {
        frame.push(0);
    }

    frame
}
