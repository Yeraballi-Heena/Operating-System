//! Integrate OS — Central Network Stack Coordinator
//!
//! Orchestrates L2 Ethernet framing, L3 ARP & IPv4 routing, L4 ICMP/UDP/TCP protocols,
//! and network device packet ingress and egress.

use alloc::vec::Vec;
use crate::net::arp::{ArpOpcode, ArpPacket, ArpTable};
use crate::net::device::{LoopbackDevice, NetDevice};
use crate::net::e1000::E1000Device;
use crate::net::ethernet::{build_ethernet_frame, EthernetHeader};
use crate::net::icmp::{build_echo_reply, build_echo_request, IcmpPacket, IcmpType};
use crate::net::interface::InterfaceConfig;
use crate::net::ipv4::{build_ipv4_packet, Ipv4Header};
use crate::net::tcp::{TcpConnection, TcpHeader, TcpState};
use crate::net::types::*;
use crate::net::udp::{build_udp_packet, UdpDatagram, UdpHeader, UdpSocketTable};
use crate::sync::IrqSpinlock;

/// Central Network Stack.
pub struct NetworkStack {
    pub config: InterfaceConfig,
    pub arp_table: ArpTable,
    pub udp_sockets: UdpSocketTable,
    pub tcp_connections: Vec<TcpConnection>,
    pub loopback: LoopbackDevice,
    pub e1000: E1000Device,
    pub next_ident: u16,
    pub ping_seq: u16,
    pub ping_replies_received: u32,
}

impl NetworkStack {
    pub const fn new() -> Self {
        Self {
            config: InterfaceConfig::virtualbox_default(),
            arp_table: ArpTable::new(),
            udp_sockets: UdpSocketTable::new(),
            tcp_connections: Vec::new(),
            loopback: LoopbackDevice::new(),
            e1000: E1000Device::new(),
            next_ident: 1,
            ping_seq: 0,
            ping_replies_received: 0,
        }
    }

    /// Allocates an incremental 16-bit IPv4 identification counter.
    pub fn next_ident(&mut self) -> u16 {
        let id = self.next_ident;
        self.next_ident = self.next_ident.wrapping_add(1);
        id
    }

    /// Polls loopback and physical network devices for incoming Ethernet frames.
    pub fn poll_ingress(&mut self) {
        let mut rx_buf = [0u8; 1514];

        // 1. Poll Loopback Device
        while self.loopback.can_receive() {
            if let Ok(len) = self.loopback.receive(&mut rx_buf) {
                if len > 0 {
                    self.process_ethernet_frame(&rx_buf[..len], true);
                }
            }
        }

        // 2. Poll E1000 Device
        while self.e1000.can_receive() {
            if let Ok(len) = self.e1000.receive(&mut rx_buf) {
                if len > 0 {
                    self.process_ethernet_frame(&rx_buf[..len], false);
                }
            }
        }
    }

    /// Processes an incoming raw Ethernet frame.
    pub fn process_ethernet_frame(&mut self, frame: &[u8], is_loopback: bool) {
        let (eth_header, payload) = match EthernetHeader::parse(frame) {
            Ok(res) => res,
            Err(_) => return,
        };

        match eth_header.ethertype {
            EtherType::Arp => {
                self.process_arp_packet(payload, is_loopback);
            }
            EtherType::Ipv4 => {
                self.process_ipv4_packet(payload, is_loopback);
            }
            _ => {}
        }
    }

    /// Processes an incoming ARP packet.
    fn process_arp_packet(&mut self, payload: &[u8], is_loopback: bool) {
        let arp = match ArpPacket::parse(payload) {
            Ok(pkt) => pkt,
            Err(_) => return,
        };

        // Cache sender's MAC mapping
        self.arp_table.insert(arp.sender_ip, arp.sender_mac, 0);

        // If it is an ARP Request for our IP, send an ARP Reply
        if arp.opcode == ArpOpcode::Request && arp.target_ip == self.config.ip {
            let my_mac = if is_loopback {
                self.loopback.mac_address()
            } else {
                self.e1000.mac_address()
            };

            let reply = ArpPacket::new_reply(my_mac, self.config.ip, arp.sender_mac, arp.sender_ip);
            let frame = build_ethernet_frame(
                arp.sender_mac,
                my_mac,
                EtherType::Arp,
                &reply.to_bytes(),
            );

            if is_loopback {
                let _ = self.loopback.send(&frame);
            } else {
                let _ = self.e1000.send(&frame);
            }
        }
    }

    /// Processes an incoming IPv4 packet.
    fn process_ipv4_packet(&mut self, payload: &[u8], is_loopback: bool) {
        let (ipv4, ip_payload) = match Ipv4Header::parse(payload) {
            Ok(res) => res,
            Err(_) => return,
        };

        // Verify destination is our IP, loopback, or broadcast
        let for_us = ipv4.dst_ip == self.config.ip
            || ipv4.dst_ip.is_loopback()
            || ipv4.dst_ip.is_broadcast();

        if !for_us {
            return;
        }

        match ipv4.protocol {
            IpProtocol::Icmp => {
                self.process_icmp_packet(&ipv4, ip_payload, is_loopback);
            }
            IpProtocol::Udp => {
                self.process_udp_packet(&ipv4, ip_payload);
            }
            IpProtocol::Tcp => {
                self.process_tcp_packet(&ipv4, ip_payload, is_loopback);
            }
            _ => {}
        }
    }

    /// Processes an incoming ICMP packet.
    fn process_icmp_packet(&mut self, ipv4: &Ipv4Header, payload: &[u8], is_loopback: bool) {
        let icmp = match IcmpPacket::parse(payload) {
            Ok(pkt) => pkt,
            Err(_) => return,
        };

        match icmp.icmp_type {
            IcmpType::EchoRequest => {
                // Generate Echo Reply
                let reply_payload = build_echo_reply(icmp.ident, icmp.seq, icmp.payload);
                let ident = self.next_ident();
                let ip_packet = build_ipv4_packet(
                    self.config.ip,
                    ipv4.src_ip,
                    IpProtocol::Icmp,
                    &reply_payload,
                    ident,
                );

                let dst_mac = if is_loopback || ipv4.src_ip.is_loopback() {
                    self.loopback.mac_address()
                } else {
                    self.arp_table.lookup(ipv4.src_ip).unwrap_or(MacAddress::BROADCAST)
                };

                let my_mac = if is_loopback {
                    self.loopback.mac_address()
                } else {
                    self.e1000.mac_address()
                };

                let frame = build_ethernet_frame(dst_mac, my_mac, EtherType::Ipv4, &ip_packet);
                if is_loopback {
                    let _ = self.loopback.send(&frame);
                } else {
                    let _ = self.e1000.send(&frame);
                }
            }
            IcmpType::EchoReply => {
                self.ping_replies_received += 1;
            }
            _ => {}
        }
    }

    /// Processes an incoming UDP datagram.
    fn process_udp_packet(&mut self, ipv4: &Ipv4Header, payload: &[u8]) {
        let (udp, udp_payload) = match UdpHeader::parse(payload, ipv4.src_ip, ipv4.dst_ip) {
            Ok(res) => res,
            Err(_) => return,
        };

        let datagram = UdpDatagram {
            src_addr: SocketAddr::new(ipv4.src_ip, udp.src_port),
            dst_addr: SocketAddr::new(ipv4.dst_ip, udp.dst_port),
            payload: Vec::from(udp_payload),
        };

        self.udp_sockets.dispatch(datagram);
    }

    /// Processes an incoming TCP segment.
    fn process_tcp_packet(&mut self, ipv4: &Ipv4Header, payload: &[u8], is_loopback: bool) {
        let (tcp, tcp_payload) = match TcpHeader::parse(payload, ipv4.src_ip, ipv4.dst_ip) {
            Ok(res) => res,
            Err(_) => return,
        };

        let local_port = tcp.dst_port;
        let remote_port = tcp.src_port;

        // Find active connection or listening socket
        let mut response_bytes = None;
        if let Some(conn) = self.tcp_connections.iter_mut().find(|c| {
            c.local_addr.port == local_port
                && (c.remote_addr.port == remote_port || c.state == TcpState::Listen)
        }) {
            if conn.state == TcpState::Listen {
                conn.remote_addr = SocketAddr::new(ipv4.src_ip, remote_port);
            }
            response_bytes = conn.process_segment(&tcp, tcp_payload, self.config.ip);
        }

        // If connection produced a response segment, send it
        if let Some(resp_tcp) = response_bytes {
            let ident = self.next_ident();
            let ip_packet = build_ipv4_packet(
                self.config.ip,
                ipv4.src_ip,
                IpProtocol::Tcp,
                &resp_tcp,
                ident,
            );

            let dst_mac = if is_loopback || ipv4.src_ip.is_loopback() {
                self.loopback.mac_address()
            } else {
                self.arp_table.lookup(ipv4.src_ip).unwrap_or(MacAddress::BROADCAST)
            };

            let my_mac = if is_loopback {
                self.loopback.mac_address()
            } else {
                self.e1000.mac_address()
            };

            let frame = build_ethernet_frame(dst_mac, my_mac, EtherType::Ipv4, &ip_packet);
            if is_loopback {
                let _ = self.loopback.send(&frame);
            } else {
                let _ = self.e1000.send(&frame);
            }
        }
    }

    /// Transmits an ICMP Echo Request (ping) to `dst_ip`.
    pub fn send_ping(&mut self, dst_ip: Ipv4Address) -> Result<u16, NetError> {
        self.ping_seq = self.ping_seq.wrapping_add(1);
        let seq = self.ping_seq;
        let ident = 0x4747;
        let payload = b"IntegrateOS Ping Probe";

        let icmp_packet = build_echo_request(ident, seq, payload);
        let ip_ident = self.next_ident();
        let ip_packet = build_ipv4_packet(
            self.config.ip,
            dst_ip,
            IpProtocol::Icmp,
            &icmp_packet,
            ip_ident,
        );

        let is_loopback = dst_ip.is_loopback() || dst_ip == self.config.ip;
        let dst_mac = if is_loopback {
            self.loopback.mac_address()
        } else {
            self.arp_table.lookup(dst_ip).unwrap_or(MacAddress::BROADCAST)
        };

        let my_mac = if is_loopback {
            self.loopback.mac_address()
        } else {
            self.e1000.mac_address()
        };

        let frame = build_ethernet_frame(dst_mac, my_mac, EtherType::Ipv4, &ip_packet);

        if is_loopback {
            self.loopback.send(&frame)?;
        } else {
            self.e1000.send(&frame)?;
        }

        Ok(seq)
    }

    /// Transmits a UDP packet to `dst_addr`.
    pub fn send_udp(
        &mut self,
        src_port: u16,
        dst_addr: SocketAddr,
        payload: &[u8],
    ) -> Result<(), NetError> {
        let src_ip = if dst_addr.ip.is_loopback() {
            Ipv4Address::LOCALHOST
        } else {
            self.config.ip
        };

        let udp_packet = build_udp_packet(
            src_ip,
            dst_addr.ip,
            src_port,
            dst_addr.port,
            payload,
        );

        let ip_ident = self.next_ident();
        let ip_packet = build_ipv4_packet(
            src_ip,
            dst_addr.ip,
            IpProtocol::Udp,
            &udp_packet,
            ip_ident,
        );

        let is_loopback = dst_addr.ip.is_loopback() || dst_addr.ip == self.config.ip;
        let dst_mac = if is_loopback {
            self.loopback.mac_address()
        } else {
            self.arp_table.lookup(dst_addr.ip).unwrap_or(MacAddress::BROADCAST)
        };

        let my_mac = if is_loopback {
            self.loopback.mac_address()
        } else {
            self.e1000.mac_address()
        };

        let frame = build_ethernet_frame(dst_mac, my_mac, EtherType::Ipv4, &ip_packet);

        if is_loopback {
            self.loopback.send(&frame)?;
        } else {
            self.e1000.send(&frame)?;
        }

        Ok(())
    }
}

/// Global Thread-Safe Network Stack Instance.
pub static NETWORK_STACK: IrqSpinlock<NetworkStack> = IrqSpinlock::new(NetworkStack::new());
