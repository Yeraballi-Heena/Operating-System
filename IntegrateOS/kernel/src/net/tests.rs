//! Integrate OS — Network Subsystem Verification Test Suite
//!
//! Validates:
//! 1. Link-layer MAC addressing, IPv4 addressing, Ethernet II framing & checksums
//! 2. RFC 826 ARP request/reply generation and dynamic ARP cache table resolution
//! 3. IPv4 packet construction, routing & automated ICMP Echo loopback (ping/pong)
//! 4. UDP connectionless datagram transmission and socket buffer multiplexing
//! 5. TCP 3-way handshake state transitions (Listen -> SynReceived -> Established) & teardown

use crate::net::arp::{ArpOpcode, ArpPacket, ArpTable};
use crate::net::checksum::{internet_checksum, transport_checksum};
use crate::net::ethernet::{build_ethernet_frame, EthernetHeader};
use crate::net::icmp::{build_echo_request, IcmpPacket, IcmpType};
use crate::net::ipv4::{build_ipv4_packet, Ipv4Header};
use crate::net::socket::{bind, close, recv_from, send_to, socket, SocketType};
use crate::net::stack::NETWORK_STACK;
use crate::net::tcp::{TcpConnection, TcpHeader, TcpState, TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_SYN};
use crate::net::types::*;
use crate::{klog_info, klog_ok};

/// Executes the full Phase 23 networking test suite.
pub fn run_net_tests() {
    klog_info!("[net-test] Starting Phase 23 Networking verification...");

    test_link_and_addressing();
    test_arp_protocol();
    test_ipv4_and_icmp_ping();
    test_udp_sockets();
    test_tcp_handshake();

    klog_ok!("[net-test] ALL PHASE 23 NETWORKING TESTS PASSED (5/5)");
}

/// Test 1: Link-layer MAC addressing, IPv4 dotted decimal parsing, framing, and RFC 1071 checksums.
fn test_link_and_addressing() {
    klog_info!("[net-test] Test 1: Link-layer, IPv4 addressing & Ethernet framing...");

    // 1. MAC Address validation
    let mac = MacAddress::parse_str("52:54:00:12:34:56").expect("Failed to parse valid MAC");
    assert_eq!(mac.0, [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    assert!(mac.is_unicast());
    assert!(!mac.is_broadcast());
    assert_eq!(mac.to_string(), "52:54:00:12:34:56");

    let bcast_mac = MacAddress::BROADCAST;
    assert!(bcast_mac.is_broadcast());
    assert!(!bcast_mac.is_unicast());

    // 2. IPv4 Address validation & Subnet check
    let ip = Ipv4Address::parse_str("192.168.1.100").expect("Failed to parse IPv4");
    assert_eq!(ip.0, [192, 168, 1, 100]);
    assert_eq!(ip.to_string(), "192.168.1.100");

    let mask = Ipv4Address::new(255, 255, 255, 0);
    let same_subnet = Ipv4Address::new(192, 168, 1, 200);
    let diff_subnet = Ipv4Address::new(192, 168, 2, 100);
    assert!(ip.is_in_subnet(same_subnet, mask));
    assert!(!ip.is_in_subnet(diff_subnet, mask));

    // 3. Ethernet Framing
    let payload = b"Hello Ethernet Layer";
    let frame_bytes = build_ethernet_frame(
        bcast_mac,
        mac,
        EtherType::Ipv4,
        payload,
    );
    assert!(frame_bytes.len() >= 60, "Frame must be padded to minimum 60 bytes");

    let (parsed_hdr, parsed_payload) = EthernetHeader::parse(&frame_bytes).expect("Failed to parse frame");
    assert_eq!(parsed_hdr.dst_mac, bcast_mac);
    assert_eq!(parsed_hdr.src_mac, mac);
    assert_eq!(parsed_hdr.ethertype, EtherType::Ipv4);
    assert!(parsed_payload.starts_with(payload));

    // 4. RFC 1071 Checksum validation
    let test_data = [0x45, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 0xac, 0x10, 0x0a, 0x63, 0xac, 0x10, 0x0a, 0x0c];
    let csum = internet_checksum(&test_data);
    assert_ne!(csum, 0);

    let mut verified_data = test_data;
    verified_data[10..12].copy_from_slice(&csum.to_be_bytes());
    assert_eq!(internet_checksum(&verified_data), 0, "Checksum verification must result in 0");

    klog_ok!("[net-test] Test 1 PASSED: Link-layer, IPv4 addressing & Ethernet framing verified");
}

/// Test 2: Address Resolution Protocol (ARP) packet serialize/parse and dynamic cache table.
fn test_arp_protocol() {
    klog_info!("[net-test] Test 2: ARP Request/Reply generation & cache resolution...");

    let host_mac = MacAddress::new(0x52, 0x54, 0x00, 0xAA, 0xBB, 0xCC);
    let host_ip = Ipv4Address::new(10, 0, 2, 15);
    let target_ip = Ipv4Address::new(10, 0, 2, 2);

    // 1. ARP Request
    let arp_req = ArpPacket::new_request(host_mac, host_ip, target_ip);
    let req_bytes = arp_req.to_bytes();
    let parsed_req = ArpPacket::parse(&req_bytes).expect("Failed to parse ARP Request");
    assert_eq!(parsed_req.opcode, ArpOpcode::Request);
    assert_eq!(parsed_req.sender_mac, host_mac);
    assert_eq!(parsed_req.sender_ip, host_ip);
    assert_eq!(parsed_req.target_ip, target_ip);

    // 2. ARP Reply
    let gw_mac = MacAddress::new(0x52, 0x54, 0x00, 0x12, 0x34, 0x02);
    let arp_rep = ArpPacket::new_reply(gw_mac, target_ip, host_mac, host_ip);
    let rep_bytes = arp_rep.to_bytes();
    let parsed_rep = ArpPacket::parse(&rep_bytes).expect("Failed to parse ARP Reply");
    assert_eq!(parsed_rep.opcode, ArpOpcode::Reply);
    assert_eq!(parsed_rep.sender_mac, gw_mac);
    assert_eq!(parsed_rep.sender_ip, target_ip);

    // 3. ARP Cache Table
    let mut table = ArpTable::new();
    assert_eq!(table.lookup(target_ip), None);

    table.insert(target_ip, gw_mac, 100);
    assert_eq!(table.lookup(target_ip), Some(gw_mac));

    // Special IP addresses
    assert!(table.lookup(Ipv4Address::LOCALHOST).is_some());
    assert_eq!(table.lookup(Ipv4Address::BROADCAST), Some(MacAddress::BROADCAST));

    table.clear();
    assert_eq!(table.lookup(target_ip), None);

    klog_ok!("[net-test] Test 2 PASSED: ARP Request/Reply and dynamic cache verified");
}

/// Test 3: IPv4 packet routing and automated ICMP Echo loopback ping diagnostics.
fn test_ipv4_and_icmp_ping() {
    klog_info!("[net-test] Test 3: IPv4 routing & ICMP ping/pong diagnostics...");

    let src = Ipv4Address::new(10, 0, 2, 15);
    let dst = Ipv4Address::new(10, 0, 2, 2);
    let test_payload = b"PING_PAYLOAD_TEST_DATA";

    // 1. IPv4 Packet Creation & Verification
    let ip_packet = build_ipv4_packet(src, dst, IpProtocol::Icmp, test_payload, 42);
    let (ipv4_hdr, ipv4_data) = Ipv4Header::parse(&ip_packet).expect("Failed to parse IPv4 packet");
    assert_eq!(ipv4_hdr.src_ip, src);
    assert_eq!(ipv4_hdr.dst_ip, dst);
    assert_eq!(ipv4_hdr.protocol, IpProtocol::Icmp);
    assert_eq!(ipv4_hdr.ident, 42);
    assert_eq!(ipv4_data, test_payload);

    // 2. ICMP Echo Request / Reply
    let echo_req = build_echo_request(0xCAFE, 1, b"PingBody");
    let parsed_icmp = IcmpPacket::parse(&echo_req).expect("Failed to parse ICMP packet");
    assert_eq!(parsed_icmp.icmp_type, IcmpType::EchoRequest);
    assert_eq!(parsed_icmp.code, 0);
    assert_eq!(parsed_icmp.ident, 0xCAFE);
    assert_eq!(parsed_icmp.seq, 1);
    assert_eq!(parsed_icmp.payload, b"PingBody");

    // 3. Live Loopback Ping Diagnostics
    let initial_replies = {
        let stack = NETWORK_STACK.lock();
        stack.ping_replies_received
    };

    // Send ping to Localhost (127.0.0.1)
    let ping_res = {
        let mut stack = NETWORK_STACK.lock();
        stack.send_ping(Ipv4Address::LOCALHOST)
    };
    assert!(ping_res.is_ok(), "Failed to send loopback ping");

    // Poll ingress to process the ping packet through loopback device
    {
        let mut stack = NETWORK_STACK.lock();
        stack.poll_ingress(); // Processes request and queues reply
        stack.poll_ingress(); // Processes reply and increments counter
    }

    let final_replies = {
        let stack = NETWORK_STACK.lock();
        stack.ping_replies_received
    };
    assert!(final_replies > initial_replies, "Expected ICMP Echo Reply to be received in loopback");

    klog_ok!("[net-test] Test 3 PASSED: IPv4 routing and ICMP ping loopback verified");
}

/// Test 4: UDP datagram transmission, pseudo-header checksum, and socket port multiplexing.
fn test_udp_sockets() {
    klog_info!("[net-test] Test 4: UDP datagram transmission & socket API...");

    let server_port = 8080;
    let client_port = 5000;
    let test_msg = b"Hello UDP Network Stack!";

    // 1. Checksum calculation over pseudo-header
    let src = Ipv4Address::LOCALHOST;
    let dst = Ipv4Address::LOCALHOST;
    let csum = transport_checksum(src, dst, 17, test_msg);
    assert_ne!(csum, 0);

    // 2. Socket API: Server binds to port 8080
    let server_sock = socket(SocketType::Udp);
    bind(server_sock, server_port).expect("Failed to bind server socket");

    // 3. Socket API: Client binds to port 5000 and sends datagram to 8080
    let client_sock = socket(SocketType::Udp);
    bind(client_sock, client_port).expect("Failed to bind client socket");

    let server_dest = SocketAddr::new(Ipv4Address::LOCALHOST, server_port);
    send_to(client_sock, server_dest, test_msg).expect("send_to failed");

    // 4. Server receives the datagram
    let mut rx_buf = [0u8; 128];
    let (bytes_read, sender_addr) = recv_from(server_sock, &mut rx_buf).expect("recv_from failed");

    assert_eq!(&rx_buf[..bytes_read], test_msg, "Received message mismatch");
    assert_eq!(sender_addr.port, client_port, "Sender port mismatch");
    assert_eq!(sender_addr.ip, Ipv4Address::LOCALHOST, "Sender IP mismatch");

    // 5. Clean up sockets
    close(server_sock);
    close(client_sock);

    klog_ok!("[net-test] Test 4 PASSED: UDP datagram and socket multiplexing verified");
}

/// Test 5: TCP 3-Way Handshake, sequence synchronization, data exchange & connection teardown.
fn test_tcp_handshake() {
    klog_info!("[net-test] Test 5: TCP 3-way handshake & state transitions...");

    let local_addr = SocketAddr::new(Ipv4Address::LOCALHOST, 80);
    let remote_addr = SocketAddr::new(Ipv4Address::LOCALHOST, 45678);

    // 1. Server listens on port 80
    let mut conn = TcpConnection::new(local_addr, remote_addr, 1000);
    conn.state = TcpState::Listen;

    // 2. Client sends SYN (seq = 500)
    let client_syn_hdr = TcpHeader::new(45678, 80, 500, 0, TCP_FLAG_SYN, 65535);
    let resp1 = conn.process_segment(&client_syn_hdr, &[], Ipv4Address::LOCALHOST);

    // Server must transition to SynReceived and produce SYN-ACK
    assert_eq!(conn.state, TcpState::SynReceived);
    assert_eq!(conn.remote_seq, 501); // ACK client's SYN (500 + 1)
    let syn_ack_bytes = resp1.expect("Server should respond with SYN-ACK");

    let (syn_ack_hdr, _) = TcpHeader::parse(&syn_ack_bytes, Ipv4Address::LOCALHOST, Ipv4Address::LOCALHOST)
        .expect("Failed to parse SYN-ACK");
    assert_eq!(syn_ack_hdr.flags & (TCP_FLAG_SYN | TCP_FLAG_ACK), TCP_FLAG_SYN | TCP_FLAG_ACK);
    assert_eq!(syn_ack_hdr.ack_num, 501);
    assert_eq!(syn_ack_hdr.seq_num, 1000);

    // 3. Client sends ACK (seq = 501, ack = 1001)
    let client_ack_hdr = TcpHeader::new(45678, 80, 501, 1001, TCP_FLAG_ACK, 65535);
    let resp2 = conn.process_segment(&client_ack_hdr, &[], Ipv4Address::LOCALHOST);
    assert!(resp2.is_none());
    assert_eq!(conn.state, TcpState::Established, "Connection must transition to Established");

    // 4. Data Transfer: Client transmits HTTP GET payload
    let http_req = b"GET /index.html HTTP/1.1\r\n\r\n";
    let client_data_hdr = TcpHeader::new(45678, 80, 501, 1001, TCP_FLAG_ACK, 65535);
    let resp3 = conn.process_segment(&client_data_hdr, http_req, Ipv4Address::LOCALHOST);

    assert_eq!(conn.rx_buffer, http_req, "Buffered payload mismatch");
    assert_eq!(conn.remote_seq, 501 + http_req.len() as u32);
    let data_ack_bytes = resp3.expect("Server should ACK payload data");
    let (data_ack_hdr, _) = TcpHeader::parse(&data_ack_bytes, Ipv4Address::LOCALHOST, Ipv4Address::LOCALHOST)
        .expect("Failed to parse data ACK");
    assert_eq!(data_ack_hdr.ack_num, 501 + http_req.len() as u32);

    // 5. Connection Teardown: Client sends FIN
    let client_fin_hdr = TcpHeader::new(45678, 80, 501 + http_req.len() as u32, 1001, TCP_FLAG_FIN, 65535);
    let resp4 = conn.process_segment(&client_fin_hdr, &[], Ipv4Address::LOCALHOST);
    assert_eq!(conn.state, TcpState::CloseWait, "Server must transition to CloseWait");
    assert!(resp4.is_some(), "Server must ACK the FIN segment");

    klog_ok!("[net-test] Test 5 PASSED: TCP 3-way handshake and state machine verified");
}
