//! Integrate OS — Audio Drivers Subsystem

pub mod ac97;
pub mod hda;
pub mod virtual_audio;

pub use ac97::Ac97Driver;
pub use hda::IntelHdaDriver;
pub use virtual_audio::VirtualAudioDriver;
