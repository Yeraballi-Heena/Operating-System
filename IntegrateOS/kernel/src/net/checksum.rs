//! Integrate OS — Internet Checksum Architecture (RFC 1071)
//!
//! Provides 16-bit one's complement addition for IPv4 headers, ICMP messages,
//! and TCP/UDP pseudo-header checksum verification and computation.

use crate::net::types::Ipv4Address;

/// Computes the standard 16-bit one's complement Internet Checksum.
pub fn internet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += word as u32;
        i += 2;
    }

    // Handle trailing odd byte
    if i < data.len() {
        let word = u16::from_be_bytes([data[i], 0]);
        sum += word as u32;
    }

    // Fold 32-bit sum into 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

/// Computes transport-layer (TCP / UDP) checksum over the IPv4 pseudo-header and payload.
pub fn transport_checksum(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    protocol: u8,
    payload: &[u8],
) -> u16 {
    let mut sum: u32 = 0;

    // 1. Source IP
    let src_bytes = src_ip.as_bytes();
    sum += u16::from_be_bytes([src_bytes[0], src_bytes[1]]) as u32;
    sum += u16::from_be_bytes([src_bytes[2], src_bytes[3]]) as u32;

    // 2. Destination IP
    let dst_bytes = dst_ip.as_bytes();
    sum += u16::from_be_bytes([dst_bytes[0], dst_bytes[1]]) as u32;
    sum += u16::from_be_bytes([dst_bytes[2], dst_bytes[3]]) as u32;

    // 3. Protocol (padded with zero)
    sum += protocol as u32;

    // 4. Transport payload length
    sum += payload.len() as u32;

    // 5. Payload bytes
    let mut i = 0;
    while i + 1 < payload.len() {
        let word = u16::from_be_bytes([payload[i], payload[i + 1]]);
        sum += word as u32;
        i += 2;
    }

    if i < payload.len() {
        let word = u16::from_be_bytes([payload[i], 0]);
        sum += word as u32;
    }

    // Fold 32-bit sum into 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    let result = !(sum as u16);
    // RFC 768: If computed checksum is 0, it is transmitted as all ones (0xFFFF) for UDP
    if result == 0 && protocol == 17 {
        0xFFFF
    } else {
        result
    }
}

/// Verifies transport-layer (TCP / UDP) checksum over the IPv4 pseudo-header and packet bytes.
pub fn verify_transport_checksum(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    protocol: u8,
    payload: &[u8],
) -> bool {
    let mut sum: u32 = 0;

    let src_bytes = src_ip.as_bytes();
    sum += u16::from_be_bytes([src_bytes[0], src_bytes[1]]) as u32;
    sum += u16::from_be_bytes([src_bytes[2], src_bytes[3]]) as u32;

    let dst_bytes = dst_ip.as_bytes();
    sum += u16::from_be_bytes([dst_bytes[0], dst_bytes[1]]) as u32;
    sum += u16::from_be_bytes([dst_bytes[2], dst_bytes[3]]) as u32;

    sum += protocol as u32;
    sum += payload.len() as u32;

    let mut i = 0;
    while i + 1 < payload.len() {
        let word = u16::from_be_bytes([payload[i], payload[i + 1]]);
        sum += word as u32;
        i += 2;
    }

    if i < payload.len() {
        let word = u16::from_be_bytes([payload[i], 0]);
        sum += word as u32;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    (sum as u16) == 0xFFFF || (sum as u16) == 0
}
