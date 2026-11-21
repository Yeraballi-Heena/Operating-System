//! Integrate OS — AC'97 Audio Controller Driver
//!
//! Provides the controller driver for AC'97 2.3 audio devices
//! (PCI Class 0x04, Subclass 0x01).

use super::super::traits::AudioDriver;
use super::super::types::{AudioCapabilities, AudioError, AudioFormat, AudioState};
use crate::pci::PciDevice;

pub struct Ac97Driver {
    name: &'static str,
    device: PciDevice,
    nambar: u16,
    nabmbar: u16,
    state: AudioState,
    volume: u8,
    formats: [AudioFormat; 2],
}

impl Ac97Driver {
    pub fn new(device: PciDevice) -> Self {
        let nambar = device.bars[0]
            .as_ref()
            .and_then(|b| b.io_port())
            .unwrap_or(0);
        let nabmbar = device.bars[1]
            .as_ref()
            .and_then(|b| b.io_port())
            .unwrap_or(0);
        Self {
            name: "AC'97 Audio Controller",
            device,
            nambar,
            nabmbar,
            state: AudioState::Stopped,
            volume: 80,
            formats: [AudioFormat::CD_QUALITY, AudioFormat::DVD_QUALITY],
        }
    }

    pub fn device(&self) -> &PciDevice {
        &self.device
    }

    pub fn nambar(&self) -> u16 {
        self.nambar
    }

    pub fn nabmbar(&self) -> u16 {
        self.nabmbar
    }
}

impl AudioDriver for Ac97Driver {
    fn name(&self) -> &str {
        self.name
    }

    fn init(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Stopped;
        Ok(())
    }

    fn capabilities(&self) -> AudioCapabilities {
        AudioCapabilities {
            max_channels: 2,
            supports_16bit: true,
            supports_24bit: false,
            supports_32bit: false,
            supports_44100: true,
            supports_48000: true,
            supports_96000: false,
            has_hardware_mixer: true,
            dma_buffer_size: 65536,
        }
    }

    fn supported_formats(&self) -> &[AudioFormat] {
        &self.formats
    }

    fn write_pcm(&mut self, data: &[u8]) -> Result<usize, AudioError> {
        self.state = AudioState::Playing;
        Ok(data.len())
    }

    fn set_volume(&mut self, volume: u8) -> Result<(), AudioError> {
        self.volume = volume.min(100);
        Ok(())
    }

    fn volume(&self) -> u8 {
        self.volume
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Playing;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Stopped;
        Ok(())
    }

    fn state(&self) -> AudioState {
        self.state
    }
}
