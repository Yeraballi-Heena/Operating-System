//! Integrate OS — Internet Control Message Protocol (ICMP, RFC 792)
//!
//! Provides Echo Request and Echo Reply packet generation and validation for network ping diagnostics.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::checksum::internet_checksum;
use crate::net::types::NetError;

pub const ICMP_HEADER_LEN: usize = 8;
pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

/// ICMP Message Type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IcmpType {
    EchoReply,
    EchoRequest,
    DestinationUnreachable,
    Unknown(u8),
}

impl IcmpType {
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::EchoReply,
            8 => Self::EchoRequest,
            3 => Self::DestinationUnreachable,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_u8(self) -> u8 {
        match self {
            Self::EchoReply => 0,
            Self::EchoRequest => 8,
            Self::DestinationUnreachable => 3,
            Self::Unknown(val) => val,
        }
    }
}

/// Parsed ICMP Packet View.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IcmpPacket<'a> {
    pub icmp_type: IcmpType,
    pub code: u8,
    pub ident: u16,
    pub seq: u16,
    pub payload: &'a [u8],
}

impl<'a> IcmpPacket<'a> {
    /// Parses an ICMP message and verifies its 16-bit Internet Checksum.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, NetError> {
        if bytes.len() < ICMP_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        // Verify Checksum
        if internet_checksum(bytes) != 0 {
            return Err(NetError::InvalidChecksum);
        }

        let icmp_type = IcmpType::from_u8(bytes[0]);
        let code = bytes[1];
        let ident = u16::from_be_bytes([bytes[4], bytes[5]]);
        let seq = u16::from_be_bytes([bytes[6], bytes[7]]);
        let payload = &bytes[ICMP_HEADER_LEN..];

        Ok(Self {
            icmp_type,
            code,
            ident,
            seq,
            payload,
        })
    }
}

/// Constructs an ICMP Echo Request (ping) packet.
pub fn build_echo_request(ident: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
    build_icmp_msg(IcmpType::EchoRequest, 0, ident, seq, payload)
}

/// Constructs an ICMP Echo Reply (pong) packet in response to a request.
pub fn build_echo_reply(ident: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
    build_icmp_msg(IcmpType::EchoReply, 0, ident, seq, payload)
}

fn build_icmp_msg(
    icmp_type: IcmpType,
    code: u8,
    ident: u16,
    seq: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut msg = vec![0u8; ICMP_HEADER_LEN + payload.len()];
    msg[0] = icmp_type.to_u8();
    msg[1] = code;
    msg[2..4].copy_from_slice(&[0, 0]); // Zero checksum for calculation
    msg[4..6].copy_from_slice(&ident.to_be_bytes());
    msg[6..8].copy_from_slice(&seq.to_be_bytes());
    msg[ICMP_HEADER_LEN..].copy_from_slice(payload);

    let csum = internet_checksum(&msg);
    msg[2..4].copy_from_slice(&csum.to_be_bytes());

    msg
}
