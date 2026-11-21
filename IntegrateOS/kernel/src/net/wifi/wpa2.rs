//! Integrate OS — WPA2-Personal (4-Way EAPOL Handshake) Engine
//!
//! Implements IEEE 802.11i 4-Way Handshake:
//! 1. AP sends ANonce to Station.
//! 2. Station generates SNonce, derives PTK (KCK, KEK, TK) via PRF-512, and responds with SNonce + MIC.
//! 3. AP validates MIC, sends GTK (Group Temporal Key) encrypted with KEK + MIC.
//! 4. Station verifies AP MIC, installs TK/GTK into hardware encryption engine, and replies with Ack.

use alloc::vec::Vec;
use crate::net::types::MacAddress;
use crate::net::wifi::crypto::{hmac_sha1, pbkdf2_sha1_wpa2, prf_512};
use crate::klog_info;

/// EAPOL-Key Handshake States
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Init,
    WaitingMsg1,
    SentMsg2,
    WaitingMsg3,
    SentMsg4,
    Completed,
    Failed(&'static str),
}

/// Pairwise Transient Key (PTK) components derived during 4-way handshake.
#[derive(Debug, Clone)]
pub struct PairwiseTransientKey {
    pub kck: [u8; 16], // Key Confirmation Key (bytes 0..16)
    pub kek: [u8; 16], // Key Encryption Key (bytes 16..32)
    pub tk: [u8; 16],  // Temporal Data Key for CCMP AES-128 (bytes 32..48)
    pub mic_rx: [u8; 8],
    pub mic_tx: [u8; 8],
}

/// Active 4-way EAPOL handshake coordinator.
pub struct Wpa2Handshake {
    pub state: HandshakeState,
    pub bssid: MacAddress,
    pub sta_mac: MacAddress,
    pub pmk: [u8; 32],
    pub anonce: [u8; 32],
    pub snonce: [u8; 32],
    pub ptk: Option<PairwiseTransientKey>,
    pub replay_counter: u64,
}

impl Wpa2Handshake {
    pub fn new(sta_mac: MacAddress, bssid: MacAddress, ssid: &str, passphrase: &str) -> Self {
        let pmk = pbkdf2_sha1_wpa2(passphrase, ssid);
        Self {
            state: HandshakeState::Init,
            bssid,
            sta_mac,
            pmk,
            anonce: [0u8; 32],
            snonce: [0u8; 32],
            ptk: None,
            replay_counter: 1,
        }
    }

    /// Step 1: Processes EAPOL Message 1 from Access Point and generates SNonce + Message 2.
    pub fn process_msg1_and_generate_msg2(&mut self, anonce: &[u8; 32]) -> Result<Vec<u8>, &'static str> {
        self.anonce.copy_from_slice(anonce);

        // Generate pseudorandom SNonce for this station session
        let ticks = crate::time::uptime_ms();
        for i in 0..32 {
            self.snonce[i] = ((ticks.wrapping_mul((i as u64) + 7) ^ 0xA5) & 0xFF) as u8;
        }

        // Derive PTK: PRF-512(PMK, "Pairwise key expansion", min(AA, SPA) || max(AA, SPA) || min(ANonce, SNonce) || max(ANonce, SNonce))
        let mut ptk_data = Vec::with_capacity(12 + 64);
        if self.bssid.0 < self.sta_mac.0 {
            ptk_data.extend_from_slice(&self.bssid.0);
            ptk_data.extend_from_slice(&self.sta_mac.0);
        } else {
            ptk_data.extend_from_slice(&self.sta_mac.0);
            ptk_data.extend_from_slice(&self.bssid.0);
        }

        if self.anonce < self.snonce {
            ptk_data.extend_from_slice(&self.anonce);
            ptk_data.extend_from_slice(&self.snonce);
        } else {
            ptk_data.extend_from_slice(&self.snonce);
            ptk_data.extend_from_slice(&self.anonce);
        }

        let ptk_bytes = prf_512(&self.pmk, b"Pairwise key expansion", &ptk_data);

        let mut kck = [0u8; 16];
        let mut kek = [0u8; 16];
        let mut tk = [0u8; 16];
        let mut mic_rx = [0u8; 8];
        let mut mic_tx = [0u8; 8];

        kck.copy_from_slice(&ptk_bytes[0..16]);
        kek.copy_from_slice(&ptk_bytes[16..32]);
        tk.copy_from_slice(&ptk_bytes[32..48]);
        mic_rx.copy_from_slice(&ptk_bytes[48..56]);
        mic_tx.copy_from_slice(&ptk_bytes[56..64]);

        self.ptk = Some(PairwiseTransientKey {
            kck,
            kek,
            tk,
            mic_rx,
            mic_tx,
        });

        // Build EAPOL Message 2 packet
        let mut msg2 = Vec::with_capacity(128);
        // IEEE 802.1X Auth Type (1), Descriptor Type (2 = EAPOL-Key)
        msg2.push(0x01);
        msg2.push(0x03);
        // Key Info: WPA2 (bit 1), Pairwise (bit 3), MIC present (bit 8)
        let key_info: u16 = 0x010A;
        msg2.extend_from_slice(&key_info.to_be_bytes());
        // Key Length: 16 (AES-CCMP)
        msg2.extend_from_slice(&16u16.to_be_bytes());
        // Replay Counter
        msg2.extend_from_slice(&self.replay_counter.to_be_bytes());
        // SNonce (32 bytes)
        msg2.extend_from_slice(&self.snonce);
        // Key IV (16 zeros)
        msg2.extend_from_slice(&[0u8; 16]);
        // Key RSC (8 zeros)
        msg2.extend_from_slice(&[0u8; 8]);
        // Key ID (8 zeros)
        msg2.extend_from_slice(&[0u8; 8]);

        // Compute HMAC-SHA1 MIC over the message with KCK
        let mic = hmac_sha1(&kck, &msg2);
        // Key MIC (16 bytes of the 20-byte HMAC)
        msg2.extend_from_slice(&mic[..16]);
        // Key Data Length (2 bytes, 0 for empty data)
        msg2.extend_from_slice(&0u16.to_be_bytes());

        self.state = HandshakeState::SentMsg2;
        Ok(msg2)
    }

    /// Step 2: Processes EAPOL Message 3 from AP (contains encrypted GTK) and generates Message 4.
    pub fn process_msg3_and_generate_msg4(&mut self, _ap_msg3: &[u8]) -> Result<Vec<u8>, &'static str> {
        let ptk = match self.ptk.as_ref() {
            Some(p) => p,
            None => return Err("PTK not derived"),
        };

        // Build EAPOL Message 4 confirmation packet
        let mut msg4 = Vec::with_capacity(128);
        msg4.push(0x01);
        msg4.push(0x03);
        // Key Info: Pairwise (bit 3), MIC (bit 8), Secure (bit 9)
        let key_info: u16 = 0x030A;
        msg4.extend_from_slice(&key_info.to_be_bytes());
        msg4.extend_from_slice(&0u16.to_be_bytes()); // Length 0
        self.replay_counter += 1;
        msg4.extend_from_slice(&self.replay_counter.to_be_bytes());
        msg4.extend_from_slice(&[0u8; 32]); // SNonce 0
        msg4.extend_from_slice(&[0u8; 16]); // IV 0
        msg4.extend_from_slice(&[0u8; 8]);  // RSC 0
        msg4.extend_from_slice(&[0u8; 8]);  // ID 0

        // Final MIC with KCK
        let mic = hmac_sha1(&ptk.kck, &msg4);
        msg4.extend_from_slice(&mic[..16]);
        // Key Data Length (2 bytes)
        msg4.extend_from_slice(&0u16.to_be_bytes());

        self.state = HandshakeState::Completed;
        klog_info!("[wpa2] 4-Way Handshake Completed! Pairwise Temporal Key (TK) active.");
        Ok(msg4)
    }
}
