//! Integrate OS — IEEE 802.11 Wireless LAN Protocol & Management Frame Engine
//!
//! Provides binary encoding, decoding, and parsing for IEEE 802.11 MAC frames:
//! - Frame Control field (Types: Management, Control, Data; Subtypes: Beacon, Probe Req/Resp, Auth, Assoc Req/Resp)
//! - 802.11 MAC Header (Receiver, Transmitter, BSSID addresses, Sequence Control)
//! - Information Element (IE) parser and serializer (SSID, Supported Rates, DS Channel, RSN/WPA2 IE)
//! - Probe Request/Response and Authentication frame construction.

use alloc::string::String;
use alloc::vec::Vec;
use crate::net::types::MacAddress;

/// IEEE 802.11 Frame Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Management,
    Control,
    Data,
    Reserved(u8),
}

/// IEEE 802.11 Frame Subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSubtype {
    // Management
    AssocRequest,
    AssocResponse,
    ReassocRequest,
    ReassocResponse,
    ProbeRequest,
    ProbeResponse,
    Beacon,
    Atim,
    Disassoc,
    Authentication,
    Deauthentication,
    Action,
    // Control
    BlockAckReq,
    BlockAck,
    PsPoll,
    Rts,
    Cts,
    Ack,
    // Data
    Data,
    DataCfaAck,
    DataCfaPoll,
    NullData,
    QosData,
    Other(u8),
}

/// IEEE 802.11 Frame Control Field (2 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameControl {
    pub protocol_version: u8,
    pub frame_type: FrameType,
    pub subtype: FrameSubtype,
    pub to_ds: bool,
    pub from_ds: bool,
    pub more_frag: bool,
    pub retry: bool,
    pub power_mgmt: bool,
    pub more_data: bool,
    pub protected: bool,
    pub order: bool,
}

impl FrameControl {
    pub fn parse(raw: u16) -> Self {
        let proto = (raw & 0x03) as u8;
        let type_val = ((raw >> 2) & 0x03) as u8;
        let subtype_val = ((raw >> 4) & 0x0F) as u8;

        let frame_type = match type_val {
            0b00 => FrameType::Management,
            0b01 => FrameType::Control,
            0b10 => FrameType::Data,
            other => FrameType::Reserved(other),
        };

        let subtype = match (frame_type, subtype_val) {
            (FrameType::Management, 0b0000) => FrameSubtype::AssocRequest,
            (FrameType::Management, 0b0001) => FrameSubtype::AssocResponse,
            (FrameType::Management, 0b0010) => FrameSubtype::ReassocRequest,
            (FrameType::Management, 0b0011) => FrameSubtype::ReassocResponse,
            (FrameType::Management, 0b0100) => FrameSubtype::ProbeRequest,
            (FrameType::Management, 0b0101) => FrameSubtype::ProbeResponse,
            (FrameType::Management, 0b1000) => FrameSubtype::Beacon,
            (FrameType::Management, 0b1001) => FrameSubtype::Atim,
            (FrameType::Management, 0b1010) => FrameSubtype::Disassoc,
            (FrameType::Management, 0b1011) => FrameSubtype::Authentication,
            (FrameType::Management, 0b1100) => FrameSubtype::Deauthentication,
            (FrameType::Management, 0b1101) => FrameSubtype::Action,
            (FrameType::Control, 0b1011) => FrameSubtype::Rts,
            (FrameType::Control, 0b1100) => FrameSubtype::Cts,
            (FrameType::Control, 0b1101) => FrameSubtype::Ack,
            (FrameType::Data, 0b0000) => FrameSubtype::Data,
            (FrameType::Data, 0b0100) => FrameSubtype::NullData,
            (FrameType::Data, 0b1000) => FrameSubtype::QosData,
            (_, other) => FrameSubtype::Other(other),
        };

        Self {
            protocol_version: proto,
            frame_type,
            subtype,
            to_ds: (raw & (1 << 8)) != 0,
            from_ds: (raw & (1 << 9)) != 0,
            more_frag: (raw & (1 << 10)) != 0,
            retry: (raw & (1 << 11)) != 0,
            power_mgmt: (raw & (1 << 12)) != 0,
            more_data: (raw & (1 << 13)) != 0,
            protected: (raw & (1 << 14)) != 0,
            order: (raw & (1 << 15)) != 0,
        }
    }

    pub fn to_raw(&self) -> u16 {
        let type_val = match self.frame_type {
            FrameType::Management => 0b00,
            FrameType::Control => 0b01,
            FrameType::Data => 0b10,
            FrameType::Reserved(v) => v & 0x03,
        };

        let subtype_val = match self.subtype {
            FrameSubtype::AssocRequest => 0b0000,
            FrameSubtype::AssocResponse => 0b0001,
            FrameSubtype::ReassocRequest => 0b0010,
            FrameSubtype::ReassocResponse => 0b0011,
            FrameSubtype::ProbeRequest => 0b0100,
            FrameSubtype::ProbeResponse => 0b0101,
            FrameSubtype::Beacon => 0b1000,
            FrameSubtype::Atim => 0b1001,
            FrameSubtype::Disassoc => 0b1010,
            FrameSubtype::Authentication => 0b1011,
            FrameSubtype::Deauthentication => 0b1100,
            FrameSubtype::Action => 0b1101,
            FrameSubtype::Rts => 0b1011,
            FrameSubtype::Cts => 0b1100,
            FrameSubtype::Ack => 0b1101,
            FrameSubtype::Data => 0b0000,
            FrameSubtype::NullData => 0b0100,
            FrameSubtype::QosData => 0b1000,
            FrameSubtype::Other(v) => v & 0x0F,
            _ => 0,
        };

        let mut val = (self.protocol_version as u16 & 0x03)
            | ((type_val as u16 & 0x03) << 2)
            | ((subtype_val as u16 & 0x0F) << 4);

        if self.to_ds { val |= 1 << 8; }
        if self.from_ds { val |= 1 << 9; }
        if self.more_frag { val |= 1 << 10; }
        if self.retry { val |= 1 << 11; }
        if self.power_mgmt { val |= 1 << 12; }
        if self.more_data { val |= 1 << 13; }
        if self.protected { val |= 1 << 14; }
        if self.order { val |= 1 << 15; }

        val
    }
}

/// Standard IEEE 802.11 MAC Frame Header (24 bytes for Management/Data)
#[derive(Debug, Clone)]
pub struct Mac80211Header {
    pub frame_control: FrameControl,
    pub duration_id: u16,
    pub addr1_receiver: MacAddress,
    pub addr2_transmitter: MacAddress,
    pub addr3_bssid: MacAddress,
    pub sequence_control: u16,
}

impl Mac80211Header {
    pub fn parse(buf: &[u8]) -> Result<(Self, &[u8]), &'static str> {
        if buf.len() < 24 {
            return Err("802.11 frame too short");
        }

        let fc_raw = u16::from_le_bytes([buf[0], buf[1]]);
        let frame_control = FrameControl::parse(fc_raw);
        let duration_id = u16::from_le_bytes([buf[2], buf[3]]);

        let mut a1 = [0u8; 6];
        let mut a2 = [0u8; 6];
        let mut a3 = [0u8; 6];
        a1.copy_from_slice(&buf[4..10]);
        a2.copy_from_slice(&buf[10..16]);
        a3.copy_from_slice(&buf[16..22]);

        let sequence_control = u16::from_le_bytes([buf[22], buf[23]]);

        let header = Self {
            frame_control,
            duration_id,
            addr1_receiver: MacAddress(a1),
            addr2_transmitter: MacAddress(a2),
            addr3_bssid: MacAddress(a3),
            sequence_control,
        };

        Ok((header, &buf[24..]))
    }

    pub fn write_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.frame_control.to_raw().to_le_bytes());
        out.extend_from_slice(&self.duration_id.to_le_bytes());
        out.extend_from_slice(&self.addr1_receiver.0);
        out.extend_from_slice(&self.addr2_transmitter.0);
        out.extend_from_slice(&self.addr3_bssid.0);
        out.extend_from_slice(&self.sequence_control.to_le_bytes());
    }
}

/// 802.11 Information Element (Tag + Length + Data)
#[derive(Debug, Clone)]
pub struct InformationElement<'a> {
    pub id: u8,
    pub data: &'a [u8],
}

impl<'a> InformationElement<'a> {
    pub const TAG_SSID: u8 = 0;
    pub const TAG_SUPPORTED_RATES: u8 = 1;
    pub const TAG_DS_PARAMETER_SET: u8 = 3;
    pub const TAG_RSN_WPA2: u8 = 48;
    pub const TAG_VENDOR_SPECIFIC: u8 = 221;
}

/// Iterator over Information Elements in an 802.11 management body.
pub struct IeIterator<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> IeIterator<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, offset: 0 }
    }
}

impl<'a> Iterator for IeIterator<'a> {
    type Item = InformationElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.buf.len() {
            return None;
        }

        let id = self.buf[self.offset];
        let len = self.buf[self.offset + 1] as usize;
        self.offset += 2;

        if self.offset + len > self.buf.len() {
            return None;
        }

        let data = &self.buf[self.offset..self.offset + len];
        self.offset += len;

        Some(InformationElement { id, data })
    }
}

/// Discovered Access Point details parsed from Beacon / Probe Response frame.
#[derive(Debug, Clone)]
pub struct ParsedBeacon {
    pub bssid: MacAddress,
    pub ssid: String,
    pub channel: u8,
    pub capabilities: u16,
    pub is_wpa2_psk: bool,
    pub is_wpa3_sae: bool,
    pub is_open: bool,
}

/// Parses an 802.11 Beacon or Probe Response frame body.
pub fn parse_beacon_or_probe_resp(
    bssid: MacAddress,
    body: &[u8],
) -> Result<ParsedBeacon, &'static str> {
    // Fixed parameters: 8 bytes Timestamp + 2 bytes Beacon Interval + 2 bytes Capability Info = 12 bytes
    if body.len() < 12 {
        return Err("Beacon frame body truncated");
    }

    let capabilities = u16::from_le_bytes([body[10], body[11]]);
    let ie_slice = &body[12..];

    let mut ssid = String::new();
    let mut channel = 1u8;
    let mut is_wpa2_psk = false;
    let mut is_wpa3_sae = false;

    for ie in IeIterator::new(ie_slice) {
        match ie.id {
            InformationElement::TAG_SSID => {
                if let Ok(s) = core::str::from_utf8(ie.data) {
                    ssid = String::from(s);
                }
            }
            InformationElement::TAG_DS_PARAMETER_SET => {
                if !ie.data.is_empty() {
                    channel = ie.data[0];
                }
            }
            InformationElement::TAG_RSN_WPA2 => {
                // RSN Information Element indicates WPA2 / WPA3
                is_wpa2_psk = true;
                // AKM suite list check for SAE (WPA3)
                if ie.data.len() >= 8 {
                    for chunk in ie.data.windows(4) {
                        // SAE OUI: 00-0F-AC:8
                        if chunk == [0x00, 0x0F, 0xAC, 0x08] {
                            is_wpa3_sae = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let is_open = !is_wpa2_psk && !is_wpa3_sae && (capabilities & (1 << 4) == 0);

    Ok(ParsedBeacon {
        bssid,
        ssid,
        channel,
        capabilities,
        is_wpa2_psk,
        is_wpa3_sae,
        is_open,
    })
}

/// Constructs a standard 802.11 Probe Request frame to broadcast across radio channels.
pub fn build_probe_request(sa: MacAddress, target_ssid: &str) -> Vec<u8> {
    let mut frame = Vec::with_capacity(64);

    let fc = FrameControl {
        protocol_version: 0,
        frame_type: FrameType::Management,
        subtype: FrameSubtype::ProbeRequest,
        to_ds: false,
        from_ds: false,
        more_frag: false,
        retry: false,
        power_mgmt: false,
        more_data: false,
        protected: false,
        order: false,
    };

    let header = Mac80211Header {
        frame_control: fc,
        duration_id: 0,
        addr1_receiver: MacAddress::BROADCAST,
        addr2_transmitter: sa,
        addr3_bssid: MacAddress::BROADCAST,
        sequence_control: 0,
    };

    header.write_to(&mut frame);

    // Information Element 0: SSID
    frame.push(InformationElement::TAG_SSID);
    frame.push(target_ssid.len() as u8);
    frame.extend_from_slice(target_ssid.as_bytes());

    // Information Element 1: Supported Rates (1, 2, 5.5, 11 Mbps)
    let rates = [0x82, 0x84, 0x8B, 0x96, 0x0C, 0x12, 0x18, 0x24];
    frame.push(InformationElement::TAG_SUPPORTED_RATES);
    frame.push(rates.len() as u8);
    frame.extend_from_slice(&rates);

    frame
}

/// Constructs an 802.11 Open System Authentication frame (Sequence 1).
pub fn build_auth_frame(sa: MacAddress, bssid: MacAddress, sequence_num: u16) -> Vec<u8> {
    let mut frame = Vec::with_capacity(32);

    let fc = FrameControl {
        protocol_version: 0,
        frame_type: FrameType::Management,
        subtype: FrameSubtype::Authentication,
        to_ds: false,
        from_ds: false,
        more_frag: false,
        retry: false,
        power_mgmt: false,
        more_data: false,
        protected: false,
        order: false,
    };

    let header = Mac80211Header {
        frame_control: fc,
        duration_id: 314,
        addr1_receiver: bssid,
        addr2_transmitter: sa,
        addr3_bssid: bssid,
        sequence_control: 0,
    };

    header.write_to(&mut frame);

    // Auth Algorithm: 0 (Open System)
    frame.extend_from_slice(&0u16.to_le_bytes());
    // Auth Transaction Sequence Number
    frame.extend_from_slice(&sequence_num.to_le_bytes());
    // Status Code: 0 (Success)
    frame.extend_from_slice(&0u16.to_le_bytes());

    frame
}

/// Constructs an 802.11 Association Request frame.
pub fn build_assoc_request(
    sa: MacAddress,
    bssid: MacAddress,
    ssid: &str,
    listen_interval: u16,
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(64);

    let fc = FrameControl {
        protocol_version: 0,
        frame_type: FrameType::Management,
        subtype: FrameSubtype::AssocRequest,
        to_ds: false,
        from_ds: false,
        more_frag: false,
        retry: false,
        power_mgmt: false,
        more_data: false,
        protected: false,
        order: false,
    };

    let header = Mac80211Header {
        frame_control: fc,
        duration_id: 314,
        addr1_receiver: bssid,
        addr2_transmitter: sa,
        addr3_bssid: bssid,
        sequence_control: 0,
    };

    header.write_to(&mut frame);

    // Capability Information: ESS (bit 0) = 1, Short Preamble (bit 5) = 1
    let cap: u16 = 0x0021;
    frame.extend_from_slice(&cap.to_le_bytes());
    // Listen Interval
    frame.extend_from_slice(&listen_interval.to_le_bytes());

    // Tag 0: SSID
    frame.push(InformationElement::TAG_SSID);
    frame.push(ssid.len() as u8);
    frame.extend_from_slice(ssid.as_bytes());

    // Tag 1: Supported Rates
    let rates = [0x82, 0x84, 0x8B, 0x96, 0x0C, 0x18, 0x30, 0x60];
    frame.push(InformationElement::TAG_SUPPORTED_RATES);
    frame.push(rates.len() as u8);
    frame.extend_from_slice(&rates);

    frame
}
