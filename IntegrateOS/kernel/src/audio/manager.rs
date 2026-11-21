//! Integrate OS — Central Audio Subsystem Manager
//!
//! Coordinates PCI audio hardware controller discovery, driver binding,
//! software audio mixing, tone synthesis, and global playback routing.

use alloc::boxed::Box;
use crate::pci::{PciDevice, PCI_MANAGER};
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};

use super::buffer::PcmBuffer;
use super::drivers::{Ac97Driver, IntelHdaDriver, VirtualAudioDriver};
use super::mixer::AudioMixer;
use super::speaker::PcSpeaker;
use super::synth::{AudioSynth, SoundPreset, Waveform};
use super::traits::AudioDriver;
use super::types::{AudioError, AudioFormat};

pub struct AudioManager {
    driver: Box<dyn AudioDriver>,
    mixer: AudioMixer,
    detected_pci: Option<PciDevice>,
}

impl AudioManager {
    pub fn new() -> Self {
        let master_fmt = AudioFormat::CD_QUALITY;
        Self {
            driver: Box::new(VirtualAudioDriver::new()),
            mixer: AudioMixer::new(master_fmt),
            detected_pci: None,
        }
    }

    /// Discovers PCI multimedia/audio controllers and binds the appropriate audio driver.
    pub fn discover_and_init(&mut self) -> Result<(), AudioError> {
        let pci_audio = {
            let mgr = PCI_MANAGER.lock();
            // Class 0x04 is Multimedia Controller
            // Subclass 0x01 = Audio Controller (AC'97)
            // Subclass 0x03 = Audio Device (Intel HD Audio)
            let mut list = mgr.find_by_class(0x04, 0x03);
            if list.is_empty() {
                list = mgr.find_by_class(0x04, 0x01);
            }
            if list.is_empty() {
                list = mgr.find_by_class(0x04, 0x00); // Video/Multimedia
            }
            list.into_iter().cloned().collect::<alloc::vec::Vec<_>>()
        };

        if let Some(dev) = pci_audio.first() {
            klog_info!(
                "[audio] Discovered PCI Audio Controller: {:04X}:{:04X} at [{}] (Class 0x{:02X}:0x{:02X})",
                dev.vendor_id,
                dev.device_id,
                dev.addr.format(),
                dev.class_code,
                dev.subclass
            );

            let driver: Box<dyn AudioDriver> = match dev.subclass {
                0x03 => {
                    klog_info!("[audio] Binding Intel High Definition Audio (HDA) Driver");
                    Box::new(IntelHdaDriver::new(dev.clone()))
                }
                0x01 => {
                    klog_info!("[audio] Binding AC'97 Audio Driver");
                    Box::new(Ac97Driver::new(dev.clone()))
                }
                _ => {
                    klog_info!("[audio] Binding Generic PCI Audio Driver");
                    Box::new(IntelHdaDriver::new(dev.clone()))
                }
            };

            self.detected_pci = Some(dev.clone());
            self.driver = driver;
        } else {
            klog_info!("[audio] No physical PCI audio card detected; binding Virtual Loopback Audio Sink");
            self.driver = Box::new(VirtualAudioDriver::new());
        }

        self.driver.init()?;
        klog_ok!(
            "[audio] Active Audio Driver: '{}' (Master Volume: {}%)",
            self.driver.name(),
            self.driver.volume()
        );

        Ok(())
    }

    /// Plays a synthesized tone through the audio pipeline.
    pub fn play_tone(&mut self, waveform: Waveform, freq_hz: f32, duration_ms: u32, volume: u8) -> u32 {
        let pcm = AudioSynth::generate_tone(waveform, freq_hz, duration_ms, volume, self.mixer.master_format);
        let track_id = self.mixer.play_buffer(pcm, volume, false);
        self.flush_mixer();
        track_id
    }

    /// Plays a preconfigured system sound preset.
    pub fn play_preset(&mut self, preset: SoundPreset, volume: u8) -> u32 {
        let pcm = AudioSynth::generate_preset(preset, volume, self.mixer.master_format);
        let track_id = self.mixer.play_buffer(pcm, volume, false);
        self.flush_mixer();
        track_id
    }

    /// Plays a raw PCM buffer through the mixer.
    pub fn play_buffer(&mut self, buffer: PcmBuffer, volume: u8, looping: bool) -> u32 {
        let track_id = self.mixer.play_buffer(buffer, volume, looping);
        self.flush_mixer();
        track_id
    }

    /// Flushes mixed audio data from the software mixer into the driver's output queue.
    pub fn flush_mixer(&mut self) {
        let mut temp_buf = [0u8; 4096];
        while self.mixer.active_tracks_count() > 0 {
            let rendered = self.mixer.render_mix(&mut temp_buf);
            if rendered == 0 {
                break;
            }
            if let Err(e) = self.driver.write_pcm(&temp_buf[..rendered]) {
                crate::klog_info!("[audio] flush_mixer warning: {}", e.as_str());
                break;
            }
        }
    }

    /// Plays a motherboard PC speaker hardware beep.
    pub fn hardware_beep(&self, freq_hz: u32, duration_ms: u32) {
        PcSpeaker::beep(freq_hz, duration_ms);
    }

    /// Sets the master system volume (0..100).
    pub fn set_master_volume(&mut self, volume: u8) {
        let vol = volume.min(100);
        self.mixer.set_master_volume(vol);
        let _ = self.driver.set_volume(vol);
    }

    /// Gets the master system volume (0..100).
    pub fn master_volume(&self) -> u8 {
        self.mixer.master_volume()
    }

    /// Sets the master mute state.
    pub fn set_muted(&mut self, muted: bool) {
        self.mixer.set_muted(muted);
    }

    /// Returns whether the audio system is currently muted.
    pub fn is_muted(&self) -> bool {
        self.mixer.is_muted()
    }

    /// Stops all currently active playback.
    pub fn stop_all(&mut self) {
        self.mixer.stop_all();
        let _ = self.driver.stop();
    }

    pub fn driver(&self) -> &dyn AudioDriver {
        &*self.driver
    }

    pub fn driver_mut(&mut self) -> &mut dyn AudioDriver {
        &mut *self.driver
    }

    pub fn mixer_mut(&mut self) -> &mut AudioMixer {
        &mut self.mixer
    }

    pub fn detected_device(&self) -> Option<&PciDevice> {
        self.detected_pci.as_ref()
    }
}

pub static AUDIO_MANAGER: IrqSpinlock<Option<AudioManager>> = IrqSpinlock::new(None);

/// Executes a closure with mutable access to the global AudioManager.
pub fn with_audio_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut AudioManager) -> R,
{
    let mut lock = AUDIO_MANAGER.lock();
    lock.as_mut().map(f)
}
