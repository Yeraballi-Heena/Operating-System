//! Integrate OS — Physical Wi-Fi & 802.11 Wireless LAN Subsystem
//!
//! Provides bare-metal driver support for the host machine's Realtek RTL8852BE
//! WiFi 6 PCIe adapter, IEEE 802.11 management frames, WPA2-PSK 4-way handshake,
//! and network interface binding.

pub mod crypto;
pub mod driver_rtl8852be;
pub mod ieee80211;
pub mod manager;
pub mod wpa2;

pub use driver_rtl8852be::{FirmwareStatus, Rtl8852beDriver};
pub use manager::{WifiAccessPoint, WifiAuth, WifiManager, WifiState};

use alloc::vec::Vec;
use crate::sync::IrqSpinlock;
use crate::klog_ok;

/// Central Wi-Fi Manager Singleton
pub static WIFI_MANAGER: IrqSpinlock<WifiManager> = IrqSpinlock::new(WifiManager::new());

/// Initializes the physical Wi-Fi hardware adapter and 802.11 subsystem.
pub fn init() {
    let mut wm = WIFI_MANAGER.lock();
    wm.init();
    let count = wm.scan().len();
    klog_ok!(
        "[wifi] Realtek RTL8852BE Wi-Fi Subsystem initialized (MAC {}, {} nearby APs detected)",
        wm.driver.hardware_mac,
        count
    );
}

/// Triggers a real 802.11 active channel scan.
pub fn scan() -> Vec<WifiAccessPoint> {
    WIFI_MANAGER.lock().scan().to_vec()
}

/// Connects to a Wi-Fi network by SSID and password.
pub fn connect(ssid: &str, password: &str) -> Result<(), &'static str> {
    WIFI_MANAGER.lock().connect(ssid, password)
}

/// Disconnects from current Wi-Fi network.
pub fn disconnect() -> Result<(), &'static str> {
    WIFI_MANAGER.lock().disconnect()
}

/// Returns true if currently associated and authenticated to Wi-Fi.
pub fn is_connected() -> bool {
    matches!(WIFI_MANAGER.lock().status(), WifiState::Connected { .. })
}
