//! Integrate OS — Internet Protocol version 4 (IPv4, RFC 791)
//!
//! Provides IPv4 packet header parsing, validation, checksum calculation, and packet creation.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::checksum::internet_checksum;
use crate::net::types::{IpProtocol, Ipv4Address, NetError};

pub const IPV4_MIN_HEADER_LEN: usize = 20;
pub const DEFAULT_TTL: u8 = 64;
pub const FLAG_DONT_FRAGMENT: u16 = 0x4000;

/// 20-byte base IPv4 Packet Header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ipv4Header {
    pub ihl: u8,
    pub tos: u8,
    pub total_length: u16,
    pub ident: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: IpProtocol,
    pub checksum: u16,
    pub src_ip: Ipv4Address,
    pub dst_ip: Ipv4Address,
}

impl Ipv4Header {
    pub fn new(
        src_ip: Ipv4Address,
        dst_ip: Ipv4Address,
        protocol: IpProtocol,
        payload_len: usize,
        ident: u16,
    ) -> Self {
        let total_length = (IPV4_MIN_HEADER_LEN + payload_len) as u16;
        Self {
            ihl: 5,
            tos: 0,
            total_length,
            ident,
            flags_fragment: FLAG_DONT_FRAGMENT,
            ttl: DEFAULT_TTL,
            protocol,
            checksum: 0,
            src_ip,
            dst_ip,
        }
    }

    /// Parses an IPv4 packet header and validates its checksum.
    pub fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), NetError> {
        if bytes.len() < IPV4_MIN_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        let ver_ihl = bytes[0];
        let version = ver_ihl >> 4;
        let ihl = ver_ihl & 0x0F;

        if version != 4 || ihl < 5 {
            return Err(NetError::PacketMalformed);
        }

        let header_len = (ihl as usize) * 4;
        if bytes.len() < header_len {
            return Err(NetError::PacketMalformed);
        }

        let tos = bytes[1];
        let total_length = u16::from_be_bytes([bytes[2], bytes[3]]);
        if (total_length as usize) > bytes.len() || (total_length as usize) < header_len {
            return Err(NetError::PacketMalformed);
        }

        // Verify Header Checksum (sum over header must be 0xFFFF)
        let calc_csum = internet_checksum(&bytes[..header_len]);
        if calc_csum != 0 {
            return Err(NetError::InvalidChecksum);
        }

        let ident = u16::from_be_bytes([bytes[4], bytes[5]]);
        let flags_frag = u16::from_be_bytes([bytes[6], bytes[7]]);
        let ttl = bytes[8];
        let proto = IpProtocol::from_u8(bytes[9]);
        let checksum = u16::from_be_bytes([bytes[10], bytes[11]]);

        let mut sip = [0u8; 4];
        sip.copy_from_slice(&bytes[12..16]);

        let mut dip = [0u8; 4];
        dip.copy_from_slice(&bytes[16..20]);

        let header = Self {
            ihl,
            tos,
            total_length,
            ident,
            flags_fragment: flags_frag,
            ttl,
            protocol: proto,
            checksum,
            src_ip: Ipv4Address(sip),
            dst_ip: Ipv4Address(dip),
        };

        let payload = &bytes[header_len..(total_length as usize)];
        Ok((header, payload))
    }

    /// Serializes and calculates the header checksum into `buf`.
    pub fn serialize(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        if buf.len() < IPV4_MIN_HEADER_LEN {
            return Err(NetError::BufferTooSmall);
        }

        buf[0] = (4 << 4) | (self.ihl & 0x0F);
        buf[1] = self.tos;
        buf[2..4].copy_from_slice(&self.total_length.to_be_bytes());
        buf[4..6].copy_from_slice(&self.ident.to_be_bytes());
        buf[6..8].copy_from_slice(&self.flags_fragment.to_be_bytes());
        buf[8] = self.ttl;
        buf[9] = self.protocol.to_u8();
        buf[10..12].copy_from_slice(&[0, 0]); // Zero checksum for calculation
        buf[12..16].copy_from_slice(self.src_ip.as_bytes());
        buf[16..20].copy_from_slice(self.dst_ip.as_bytes());

        // Calculate and write real checksum
        let csum = internet_checksum(&buf[..IPV4_MIN_HEADER_LEN]);
        buf[10..12].copy_from_slice(&csum.to_be_bytes());

        Ok(IPV4_MIN_HEADER_LEN)
    }
}

/// Constructs a complete IPv4 packet with calculated checksum.
pub fn build_ipv4_packet(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    protocol: IpProtocol,
    payload: &[u8],
    ident: u16,
) -> Vec<u8> {
    let header = Ipv4Header::new(src_ip, dst_ip, protocol, payload.len(), ident);
    let mut packet = vec![0u8; IPV4_MIN_HEADER_LEN + payload.len()];
    let _ = header.serialize(&mut packet[..IPV4_MIN_HEADER_LEN]);
    packet[IPV4_MIN_HEADER_LEN..].copy_from_slice(payload);
    packet
}
