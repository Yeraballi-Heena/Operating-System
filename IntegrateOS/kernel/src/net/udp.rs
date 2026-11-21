//! Integrate OS — User Datagram Protocol (UDP, RFC 768)
//!
//! Provides connectionless, unreliable datagram transmission and port multiplexing.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::checksum::{transport_checksum, verify_transport_checksum};
use crate::net::types::{Ipv4Address, NetError, SocketAddr};

pub const UDP_HEADER_LEN: usize = 8;

/// 8-byte RFC 768 UDP Header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub const fn new(src_port: u16, dst_port: u16, payload_len: usize) -> Self {
        Self {
            src_port,
            dst_port,
            length: (UDP_HEADER_LEN + payload_len) as u16,
            checksum: 0,
        }
    }

    /// Parses a UDP datagram and optionally verifies its pseudo-header checksum.
    pub fn parse(
        bytes: &[u8],
        src_ip: Ipv4Address,
        dst_ip: Ipv4Address,
    ) -> Result<(Self, &[u8]), NetError> {
        if bytes.len() < UDP_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        let src_port = u16::from_be_bytes([bytes[0], bytes[1]]);
        let dst_port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let length = u16::from_be_bytes([bytes[4], bytes[5]]);
        let checksum = u16::from_be_bytes([bytes[6], bytes[7]]);

        if (length as usize) > bytes.len() || (length as usize) < UDP_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        let payload = &bytes[UDP_HEADER_LEN..(length as usize)];

        // If checksum is non-zero, verify against IPv4 pseudo header
        if checksum != 0 && !verify_transport_checksum(src_ip, dst_ip, 17, bytes) {
            return Err(NetError::InvalidChecksum);
        }

        let header = Self {
            src_port,
            dst_port,
            length,
            checksum,
        };

        Ok((header, payload))
    }
}

/// A received UDP Datagram with origin and destination endpoint addresses.
#[derive(Clone, Debug)]
pub struct UdpDatagram {
    pub src_addr: SocketAddr,
    pub dst_addr: SocketAddr,
    pub payload: Vec<u8>,
}

/// Constructs a complete UDP packet including IPv4 pseudo-header checksum.
pub fn build_udp_packet(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut packet = vec![0u8; UDP_HEADER_LEN + payload.len()];
    packet[0..2].copy_from_slice(&src_port.to_be_bytes());
    packet[2..4].copy_from_slice(&dst_port.to_be_bytes());
    let total_len = (UDP_HEADER_LEN + payload.len()) as u16;
    packet[4..6].copy_from_slice(&total_len.to_be_bytes());
    packet[6..8].copy_from_slice(&[0, 0]); // Zero checksum for calculation
    packet[UDP_HEADER_LEN..].copy_from_slice(payload);

    let csum = transport_checksum(src_ip, dst_ip, 17, &packet);
    packet[6..8].copy_from_slice(&csum.to_be_bytes());

    packet
}

/// UDP Socket Bindings Table.
pub struct UdpSocketTable {
    pub sockets: Vec<BoundUdpSocket>,
}

pub struct BoundUdpSocket {
    pub port: u16,
    pub rx_queue: Vec<UdpDatagram>,
}

impl UdpSocketTable {
    pub const fn new() -> Self {
        Self {
            sockets: Vec::new(),
        }
    }

    /// Binds a local port to receive incoming UDP datagrams.
    pub fn bind(&mut self, port: u16) -> Result<(), NetError> {
        if self.sockets.iter().any(|s| s.port == port) {
            return Err(NetError::PortInUse);
        }

        self.sockets.push(BoundUdpSocket {
            port,
            rx_queue: Vec::new(),
        });
        Ok(())
    }

    /// Delivers an incoming datagram to the socket bound on `dst_port`.
    pub fn dispatch(&mut self, datagram: UdpDatagram) -> bool {
        if let Some(sock) = self.sockets.iter_mut().find(|s| s.port == datagram.dst_addr.port) {
            // Buffer up to 32 datagrams per socket
            if sock.rx_queue.len() < 32 {
                sock.rx_queue.push(datagram);
            }
            true
        } else {
            false
        }
    }

    /// Receives a queued datagram from `port`.
    pub fn recv_from(&mut self, port: u16) -> Option<UdpDatagram> {
        if let Some(sock) = self.sockets.iter_mut().find(|s| s.port == port) {
            if !sock.rx_queue.is_empty() {
                return Some(sock.rx_queue.remove(0));
            }
        }
        None
    }

    /// Closes and unbinds a port.
    pub fn close(&mut self, port: u16) {
        self.sockets.retain(|s| s.port != port);
    }
}
