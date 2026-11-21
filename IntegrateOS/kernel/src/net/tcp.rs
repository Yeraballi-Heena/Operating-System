//! Integrate OS — Transmission Control Protocol (TCP, RFC 793)
//!
//! Provides reliable, stream-oriented, connection-based transport with full
//! 3-way handshake (SYN, SYN-ACK, ACK), sequence tracking, and connection teardown.

use alloc::vec;
use alloc::vec::Vec;
use crate::net::checksum::{transport_checksum, verify_transport_checksum};
use crate::net::types::{Ipv4Address, NetError, SocketAddr};

pub const TCP_MIN_HEADER_LEN: usize = 20;

// TCP Control Flags
pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_PSH: u8 = 0x08;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;

/// RFC 793 TCP Connection States.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// 20-byte base RFC 793 TCP Header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8, // in 32-bit words (min 5)
    pub flags: u8,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}

impl TcpHeader {
    pub fn new(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window_size: u16,
    ) -> Self {
        Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset: 5,
            flags,
            window_size,
            checksum: 0,
            urgent_ptr: 0,
        }
    }

    /// Parses a TCP packet and verifies its pseudo-header checksum.
    pub fn parse(
        bytes: &[u8],
        src_ip: Ipv4Address,
        dst_ip: Ipv4Address,
    ) -> Result<(Self, &[u8]), NetError> {
        if bytes.len() < TCP_MIN_HEADER_LEN {
            return Err(NetError::PacketMalformed);
        }

        // Verify Checksum against IPv4 pseudo header
        if !verify_transport_checksum(src_ip, dst_ip, 6, bytes) {
            return Err(NetError::InvalidChecksum);
        }

        let src_port = u16::from_be_bytes([bytes[0], bytes[1]]);
        let dst_port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let seq_num = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ack_num = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let data_offset = (bytes[12] >> 4) & 0x0F;
        let flags = bytes[13] & 0x3F;
        let window_size = u16::from_be_bytes([bytes[14], bytes[15]]);
        let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
        let urgent_ptr = u16::from_be_bytes([bytes[18], bytes[19]]);

        let header_len = (data_offset as usize) * 4;
        if header_len < TCP_MIN_HEADER_LEN || header_len > bytes.len() {
            return Err(NetError::PacketMalformed);
        }

        let payload = &bytes[header_len..];

        let header = Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window_size,
            checksum,
            urgent_ptr,
        };

        Ok((header, payload))
    }
}

/// Constructs a complete TCP segment including IPv4 pseudo-header checksum.
pub fn build_tcp_packet(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    flags: u8,
    window_size: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut packet = vec![0u8; TCP_MIN_HEADER_LEN + payload.len()];
    packet[0..2].copy_from_slice(&src_port.to_be_bytes());
    packet[2..4].copy_from_slice(&dst_port.to_be_bytes());
    packet[4..8].copy_from_slice(&seq_num.to_be_bytes());
    packet[8..12].copy_from_slice(&ack_num.to_be_bytes());
    packet[12] = 5 << 4; // Data offset: 5 dwords (20 bytes)
    packet[13] = flags;
    packet[14..16].copy_from_slice(&window_size.to_be_bytes());
    packet[16..18].copy_from_slice(&[0, 0]); // Zero checksum for calculation
    packet[18..20].copy_from_slice(&[0, 0]); // Urgent pointer
    packet[TCP_MIN_HEADER_LEN..].copy_from_slice(payload);

    let csum = transport_checksum(src_ip, dst_ip, 6, &packet);
    packet[16..18].copy_from_slice(&csum.to_be_bytes());

    packet
}

/// An active or listening TCP connection endpoint.
#[derive(Clone, Debug)]
pub struct TcpConnection {
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub state: TcpState,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub rx_buffer: Vec<u8>,
}

impl TcpConnection {
    pub fn new(local_addr: SocketAddr, remote_addr: SocketAddr, initial_seq: u32) -> Self {
        Self {
            local_addr,
            remote_addr,
            state: TcpState::Closed,
            local_seq: initial_seq,
            remote_seq: 0,
            rx_buffer: Vec::new(),
        }
    }

    /// Advances state upon receiving an incoming TCP segment.
    /// Returns an optional response packet bytes to transmit.
    pub fn process_segment(
        &mut self,
        header: &TcpHeader,
        payload: &[u8],
        local_ip: Ipv4Address,
    ) -> Option<Vec<u8>> {
        match self.state {
            TcpState::Listen => {
                // Incoming SYN
                if (header.flags & TCP_FLAG_SYN) != 0 {
                    self.remote_seq = header.seq_num.wrapping_add(1);
                    self.state = TcpState::SynReceived;

                    // Respond with SYN + ACK
                    let resp = build_tcp_packet(
                        local_ip,
                        self.remote_addr.ip,
                        self.local_addr.port,
                        self.remote_addr.port,
                        self.local_seq,
                        self.remote_seq,
                        TCP_FLAG_SYN | TCP_FLAG_ACK,
                        65535,
                        &[],
                    );
                    self.local_seq = self.local_seq.wrapping_add(1);
                    return Some(resp);
                }
            }
            TcpState::SynSent => {
                // Expecting SYN + ACK
                if (header.flags & (TCP_FLAG_SYN | TCP_FLAG_ACK)) == (TCP_FLAG_SYN | TCP_FLAG_ACK) {
                    if header.ack_num == self.local_seq {
                        self.remote_seq = header.seq_num.wrapping_add(1);
                        self.state = TcpState::Established;

                        // Respond with ACK
                        let resp = build_tcp_packet(
                            local_ip,
                            self.remote_addr.ip,
                            self.local_addr.port,
                            self.remote_addr.port,
                            self.local_seq,
                            self.remote_seq,
                            TCP_FLAG_ACK,
                            65535,
                            &[],
                        );
                        return Some(resp);
                    }
                }
            }
            TcpState::SynReceived => {
                // Expecting ACK of our SYN-ACK
                if (header.flags & TCP_FLAG_ACK) != 0 && header.ack_num == self.local_seq {
                    self.state = TcpState::Established;
                }
            }
            TcpState::Established => {
                // Receive incoming payload data
                if !payload.is_empty() {
                    self.rx_buffer.extend_from_slice(payload);
                    self.remote_seq = self.remote_seq.wrapping_add(payload.len() as u32);

                    // Send ACK back
                    let resp = build_tcp_packet(
                        local_ip,
                        self.remote_addr.ip,
                        self.local_addr.port,
                        self.remote_addr.port,
                        self.local_seq,
                        self.remote_seq,
                        TCP_FLAG_ACK,
                        65535,
                        &[],
                    );
                    return Some(resp);
                }

                // If FIN received, initiate teardown
                if (header.flags & TCP_FLAG_FIN) != 0 {
                    self.remote_seq = self.remote_seq.wrapping_add(1);
                    self.state = TcpState::CloseWait;

                    // Send ACK for the FIN
                    let resp = build_tcp_packet(
                        local_ip,
                        self.remote_addr.ip,
                        self.local_addr.port,
                        self.remote_addr.port,
                        self.local_seq,
                        self.remote_seq,
                        TCP_FLAG_ACK,
                        65535,
                        &[],
                    );
                    return Some(resp);
                }
            }
            TcpState::CloseWait => {
                // Application closes -> sends FIN
            }
            TcpState::FinWait1 => {
                if (header.flags & TCP_FLAG_ACK) != 0 {
                    self.state = TcpState::FinWait2;
                }
            }
            TcpState::FinWait2 => {
                if (header.flags & TCP_FLAG_FIN) != 0 {
                    self.remote_seq = self.remote_seq.wrapping_add(1);
                    self.state = TcpState::TimeWait;

                    let resp = build_tcp_packet(
                        local_ip,
                        self.remote_addr.ip,
                        self.local_addr.port,
                        self.remote_addr.port,
                        self.local_seq,
                        self.remote_seq,
                        TCP_FLAG_ACK,
                        65535,
                        &[],
                    );
                    return Some(resp);
                }
            }
            _ => {}
        }
        None
    }
}
