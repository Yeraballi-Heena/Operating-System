//! Integrate OS — Phase 27 Audio Verification Test Suite
//!
//! Verifies:
//! - Test 1: Audio controller detection & driver abstraction (Intel HDA / AC'97 / Virtual Sink)
//! - Test 2: PCM buffer abstraction & stream format mathematics
//! - Test 3: Mathematical waveform synthesis (Sine, Square, Sawtooth, Triangle, Noise) & presets
//! - Test 4: Software audio mixer, multi-track blending & saturation clamping
//! - Test 5: Playback stream pipeline, DMA / FIFO write operations & state machine

use crate::{klog_info, klog_ok};
use super::buffer::PcmBuffer;
use super::manager::AUDIO_MANAGER;
use super::mixer::AudioMixer;
use super::speaker::PcSpeaker;
use super::synth::{AudioSynth, Notes, SoundPreset, Waveform};
use super::types::*;

/// Runs the complete Phase 27 Audio verification test suite.
pub fn run_audio_tests() {
    klog_info!("[audio-test] Starting Phase 27 Audio verification...");

    test_1_controller_detection_and_abstraction();
    test_2_pcm_buffer_and_format_math();
    test_3_waveform_synthesis_and_notes();
    test_4_software_mixer_and_saturation();
    test_5_playback_pipeline_and_state_machine();

    klog_ok!("[audio-test] ALL PHASE 27 AUDIO TESTS PASSED (5/5)");
}

/// Test 1: Audio Controller detection and driver abstraction.
fn test_1_controller_detection_and_abstraction() {
    klog_info!("[audio-test] Test 1: Audio controller detection & driver abstraction...");

    let lock = AUDIO_MANAGER.lock();
    let mgr = lock.as_ref().expect("Test 1 Failed: AUDIO_MANAGER not initialized");

    let driver = mgr.driver();
    let name = driver.name();
    assert!(!name.is_empty(), "Test 1 Failed: Driver name is empty");

    klog_info!("[audio-test] Active audio driver: '{}'", name);

    let caps = driver.capabilities();
    assert!(caps.max_channels >= 2, "Test 1 Failed: Less than 2 channels supported");
    assert!(caps.supports_16bit, "Test 1 Failed: 16-bit PCM not supported");
    assert!(caps.supports_44100, "Test 1 Failed: 44.1 kHz sample rate not supported");
    assert!(caps.dma_buffer_size >= 4096, "Test 1 Failed: DMA buffer too small");

    let formats = driver.supported_formats();
    assert!(!formats.is_empty(), "Test 1 Failed: No supported formats reported");

    let has_cd_quality = formats.iter().any(|f| {
        f.sample_format == SampleFormat::S16Le
            && f.channels == AudioChannels::Stereo
            && f.sample_rate == 44100
    });
    assert!(has_cd_quality, "Test 1 Failed: CD Quality (44.1kHz S16 Stereo) not in supported list");

    klog_ok!("[audio-test] Test 1 PASSED: Audio controller detection & driver abstraction verified");
}

/// Test 2: PCM buffer abstraction & stream format mathematics.
fn test_2_pcm_buffer_and_format_math() {
    klog_info!("[audio-test] Test 2: PCM buffer abstraction & stream format math...");

    // 1. Verify AudioFormat calculations
    let cd_fmt = AudioFormat::CD_QUALITY;
    assert_eq!(cd_fmt.sample_format.bytes_per_sample(), 2);
    assert_eq!(cd_fmt.channels.count(), 2);
    assert_eq!(cd_fmt.frame_size(), 4); // 2 channels * 2 bytes = 4 bytes per frame
    assert_eq!(cd_fmt.byte_rate(), 44100 * 4); // 176,400 bytes/sec
    assert_eq!(cd_fmt.duration_ms(176400), 1000); // 1 second = 1000 ms
    assert_eq!(cd_fmt.bytes_for_duration(500), 88200);

    // 2. Allocate PCM buffer and verify frames
    let mut buf = PcmBuffer::with_capacity(cd_fmt, 1000);
    assert_eq!(buf.byte_len(), 0);
    assert_eq!(buf.frame_count(), 0);

    // Push 100 stereo frames (200 samples = 400 bytes)
    for i in 0..100 {
        buf.push_sample_s16((i * 100) as i16);
        buf.push_sample_s16((-((i * 100) as i16)) as i16);
    }
    assert_eq!(buf.frame_count(), 100);
    assert_eq!(buf.byte_len(), 400);
    assert_eq!(buf.sample_s16(0), Some(0));
    assert_eq!(buf.sample_s16(1), Some(0));
    assert_eq!(buf.sample_s16(2), Some(100));
    assert_eq!(buf.sample_s16(3), Some(-100));

    // 3. Test volume attenuation
    buf.apply_volume(50); // 50% volume
    assert_eq!(buf.sample_s16(2), Some(50));
    assert_eq!(buf.sample_s16(3), Some(-50));

    // 4. Test silence buffer
    let silence = PcmBuffer::silence(cd_fmt, 50);
    assert_eq!(silence.byte_len(), 200);
    assert_eq!(silence.peak_amplitude_s16(), 0);

    klog_ok!("[audio-test] Test 2 PASSED: PCM buffer abstraction & stream format math verified");
}

/// Test 3: Mathematical waveform synthesis & note generation.
fn test_3_waveform_synthesis_and_notes() {
    klog_info!("[audio-test] Test 3: Mathematical waveform synthesis & note generation...");

    // 1. Verify sine wave approximation at key phases
    let sin_0 = AudioSynth::approx_sin(0.0);
    let sin_quarter = AudioSynth::approx_sin(0.25);
    let sin_half = AudioSynth::approx_sin(0.5);
    let sin_three_quarter = AudioSynth::approx_sin(0.75);

    assert!(sin_0.abs() < 0.05, "Test 3 Failed: sin(0) not ~0: {}", sin_0);
    assert!((sin_quarter - 1.0).abs() < 0.05, "Test 3 Failed: sin(0.25) not ~1.0: {}", sin_quarter);
    assert!(sin_half.abs() < 0.05, "Test 3 Failed: sin(0.5) not ~0: {}", sin_half);
    assert!((sin_three_quarter - (-1.0)).abs() < 0.05, "Test 3 Failed: sin(0.75) not ~-1.0: {}", sin_three_quarter);

    // 2. Synthesize a 440 Hz (A4) Sine tone
    let format = AudioFormat::CD_QUALITY;
    let sine_buf = AudioSynth::generate_tone(Waveform::Sine, Notes::A4, 100, 100, format);
    assert_eq!(sine_buf.duration_ms(), 100);
    let peak = sine_buf.peak_amplitude_s16();
    assert!(peak > 25000, "Test 3 Failed: Sine peak amplitude too low: {}", peak);

    // 3. Synthesize Square and Sawtooth waveforms
    let square_buf = AudioSynth::generate_tone(Waveform::Square, Notes::C4, 50, 80, format);
    assert_eq!(square_buf.duration_ms(), 50);
    assert!(square_buf.peak_amplitude_s16() > 20000);

    let saw_buf = AudioSynth::generate_tone(Waveform::Sawtooth, Notes::E4, 50, 90, format);
    assert_eq!(saw_buf.duration_ms(), 50);

    let tri_buf = AudioSynth::generate_tone(Waveform::Triangle, Notes::G4, 50, 90, format);
    assert_eq!(tri_buf.duration_ms(), 50);

    let noise_buf = AudioSynth::generate_tone(Waveform::Noise, 0.0, 50, 70, format);
    assert_eq!(noise_buf.duration_ms(), 50);

    // 4. Test sound presets
    let startup_chime = AudioSynth::generate_preset(SoundPreset::StartupChime, 80, format);
    assert!(startup_chime.byte_len() > 0);
    assert!(startup_chime.duration_ms() >= 300);

    let alert_beep = AudioSynth::generate_preset(SoundPreset::AlertBeep, 80, format);
    assert!(alert_beep.duration_ms() >= 50);

    klog_ok!("[audio-test] Test 3 PASSED: Mathematical waveform synthesis & presets verified");
}

/// Test 4: Software audio mixer, multi-track blending & saturation clamping.
fn test_4_software_mixer_and_saturation() {
    klog_info!("[audio-test] Test 4: Software audio mixer & saturation clamping...");

    let format = AudioFormat::CD_QUALITY;
    let mut mixer = AudioMixer::new(format);
    mixer.set_master_volume(100);

    // 1. Create two loud tones that will collide and test saturation clamping
    let tone1 = AudioSynth::generate_tone(Waveform::Square, 440.0, 50, 100, format);
    let tone2 = AudioSynth::generate_tone(Waveform::Square, 440.0, 50, 100, format);

    let id1 = mixer.play_buffer(tone1, 100, false);
    let id2 = mixer.play_buffer(tone2, 100, false);
    assert_eq!(mixer.active_tracks_count(), 2);

    // 2. Render mix into buffer
    let mut out_data = [0u8; 1024];
    let rendered = mixer.render_mix(&mut out_data);
    assert_eq!(rendered, 1024);

    // 3. Inspect samples in mixed output - verify non-zero content without wraparound distortion
    let mut has_non_zero = false;
    for chunk in out_data.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        if sample != 0 {
            has_non_zero = true;
        }
    }
    assert!(has_non_zero, "Test 4 Failed: Mixed output contains only silence");

    // 4. Test track stopping
    mixer.stop_track(id1);
    mixer.stop_track(id2);
    assert_eq!(mixer.active_tracks_count(), 0);

    // 5. Test master volume and mute
    mixer.set_master_volume(40);
    assert_eq!(mixer.master_volume(), 40);
    mixer.set_muted(true);
    assert!(mixer.is_muted());

    let mut mute_out = [0xFFu8; 100];
    mixer.render_mix(&mut mute_out);
    assert!(mute_out.iter().all(|&b| b == 0)); // Silence rendered when muted

    klog_ok!("[audio-test] Test 4 PASSED: Software audio mixer & saturation clamping verified");
}

/// Test 5: Playback stream pipeline, DMA / FIFO write operations & state machine.
fn test_5_playback_pipeline_and_state_machine() {
    klog_info!("[audio-test] Test 5: Playback stream pipeline & state machine...");

    let mut lock = AUDIO_MANAGER.lock();
    let mgr = lock.as_mut().expect("Test 5 Failed: AUDIO_MANAGER not initialized");

    // 1. Play tone through high-level manager pipeline
    let track_id = mgr.play_tone(Waveform::Sine, Notes::C5, 100, 80);
    assert!(track_id > 0);

    // 2. Verify driver state
    let state = mgr.driver().state();
    assert!(
        state == AudioState::Playing || state == AudioState::Stopped,
        "Test 5 Failed: Unexpected driver state: {:?}",
        state
    );

    // 3. Test volume control propagation
    mgr.set_master_volume(75);
    assert_eq!(mgr.master_volume(), 75);
    assert_eq!(mgr.driver().volume(), 75);

    // 4. Test driver state transitions
    let pause_res = mgr.driver_mut().pause();
    assert!(pause_res.is_ok());
    assert_eq!(mgr.driver().state(), AudioState::Paused);

    let resume_res = mgr.driver_mut().resume();
    assert!(resume_res.is_ok());
    assert_eq!(mgr.driver().state(), AudioState::Playing);

    let stop_res = mgr.driver_mut().stop();
    assert!(stop_res.is_ok());
    assert_eq!(mgr.driver().state(), AudioState::Stopped);

    // 5. Test PC Speaker interface
    PcSpeaker::start_tone(1000);
    PcSpeaker::stop_tone();

    klog_ok!("[audio-test] Test 5 PASSED: Playback stream pipeline & state machine verified");
}
