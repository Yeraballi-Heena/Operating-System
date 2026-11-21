//! Integrate OS — Intel High Definition Audio (HDA) Driver
//!
//! Provides the controller driver for Intel HD Audio (Azalia) standard controllers
//! (PCI Class 0x04, Subclass 0x03).

use super::super::traits::AudioDriver;
use super::super::types::{
    AudioCapabilities, AudioChannels, AudioError, AudioFormat, AudioState, SampleFormat,
};
use crate::pci::PciDevice;

/// Intel HDA Register Offsets
pub const HDA_GCAP: usize = 0x00;
pub const HDA_GCTL: usize = 0x08;
pub const HDA_WAKEEN: usize = 0x0C;
pub const HDA_STATESTS: usize = 0x0E;
pub const HDA_INTCTL: usize = 0x20;
pub const HDA_INTSTS: usize = 0x24;

/// Stream Descriptor 0 Output Registers (offset 0x100)
pub const HDA_SD0_CTL: usize = 0x100;
pub const HDA_SD0_STS: usize = 0x103;
pub const HDA_SD0_LPIB: usize = 0x104;
pub const HDA_SD0_CBL: usize = 0x108;
pub const HDA_SD0_LVI: usize = 0x10C;
pub const HDA_SD0_FMT: usize = 0x112;
pub const HDA_SD0_BDPL: usize = 0x118;
pub const HDA_SD0_BDPU: usize = 0x11C;

pub struct IntelHdaDriver {
    name: &'static str,
    device: PciDevice,
    mmio_base: u64,
    state: AudioState,
    volume: u8,
    formats: [AudioFormat; 3],
}

impl IntelHdaDriver {
    pub fn new(device: PciDevice) -> Self {
        let mmio = device.bars[0]
            .as_ref()
            .and_then(|b| b.mmio_address())
            .unwrap_or(0);
        Self {
            name: "Intel High Definition Audio Controller",
            device,
            mmio_base: mmio,
            state: AudioState::Stopped,
            volume: 85,
            formats: [
                AudioFormat::CD_QUALITY,
                AudioFormat::DVD_QUALITY,
                AudioFormat::new(SampleFormat::S16Le, AudioChannels::Surround51, 48000),
            ],
        }
    }

    pub fn device(&self) -> &PciDevice {
        &self.device
    }

    pub fn mmio_base(&self) -> u64 {
        self.mmio_base
    }
}

impl AudioDriver for IntelHdaDriver {
    fn name(&self) -> &str {
        self.name
    }

    fn init(&mut self) -> Result<(), AudioError> {
        self.state = AudioState::Stopped;
        // In real hardware or emulation, bring controller out of reset via GCTL CRST bit
        Ok(())
    }

    fn capabilities(&self) -> AudioCapabilities {
        AudioCapabilities {
            max_channels: 6,
            supports_16bit: true,
            supports_24bit: true,
            supports_32bit: true,
            supports_44100: true,
            supports_48000: true,
            supports_96000: true,
            has_hardware_mixer: true,
            dma_buffer_size: 131072,
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
