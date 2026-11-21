//! Integrate OS — Audio Subsystem Architecture
//!
//! Provides PCI audio controller detection (Intel HDA / AC'97), PCM buffer abstraction,
//! mathematical waveform synthesis, multi-track software audio mixing, PC speaker support,
//! and global playback control.

pub mod buffer;
pub mod drivers;
pub mod manager;
pub mod mixer;
pub mod speaker;
pub mod synth;
pub mod tests;
pub mod traits;
pub mod types;

pub use buffer::PcmBuffer;
pub use manager::{with_audio_mut, AudioManager, AUDIO_MANAGER};
pub use mixer::{AudioMixer, MixerTrack};
pub use speaker::PcSpeaker;
pub use synth::{AudioSynth, Notes, SoundPreset, Waveform};
pub use traits::AudioDriver;
pub use types::*;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::klog_ok;

static AUDIO_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` if the audio subsystem has successfully initialized.
pub fn is_audio_initialized() -> bool {
    AUDIO_INITIALIZED.load(Ordering::Relaxed)
}

/// Initializes the audio subsystem, probes PCI audio hardware, and sets up the global audio manager.
pub fn init() {
    let mut mgr = AudioManager::new();
    let result = mgr.discover_and_init();

    if let Err(e) = result {
        crate::klog_info!("[audio] Hardware audio init notice: {}", e.as_str());
    }

    *AUDIO_MANAGER.lock() = Some(mgr);
    AUDIO_INITIALIZED.store(true, Ordering::SeqCst);
    klog_ok!("[audio] Audio Subsystem initialized");
}
