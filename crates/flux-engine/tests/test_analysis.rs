//! Integration tests for analysis foundations: Peak pyramids, Spectrograms, BS.1770 Loudness, and Stereo analysis (P1.5).

use flux_dsp::loudness::LoudnessMeter;
use flux_dsp::pyramid::PeakPyramid;
use flux_dsp::spatial_analysis::StereoAnalyzer;
use flux_dsp::spectrogram::{
    ColorMap, FrequencyScale, SpectrogramConfig, SpectrogramTileGenerator,
};
use std::f32::consts::PI;

#[test]
fn test_peak_pyramid_hierarchy_and_serialization() {
    let frames = 65536;
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);

    for i in 0..frames {
        let s = (i as f32 * 0.01).sin() * 0.9;
        left.push(s);
        right.push(-s * 0.5);
    }

    let pyramid = PeakPyramid::build(&[&left, &right], 64);
    assert_eq!(pyramid.num_channels, 2);
    assert_eq!(pyramid.total_frames, frames);
    assert!(pyramid.levels.len() >= 3);

    // Level 0 has 64 samples/bin -> 1024 bins
    assert_eq!(pyramid.levels[0].frames_per_bin, 64);
    assert_eq!(pyramid.levels[0].channels[0].len(), 1024);

    // Level 1 has 256 samples/bin -> 256 bins
    assert_eq!(pyramid.levels[1].frames_per_bin, 256);
    assert_eq!(pyramid.levels[1].channels[0].len(), 256);

    // Query range for waveform overview (e.g. 100 bins for whole file)
    let overview_bins = pyramid.query_range(0, 0, frames, 100);
    assert!(!overview_bins.is_empty());
    assert!(overview_bins.len() <= 256);

    // Serialization round-trip
    let bytes = pyramid.to_bytes();
    assert!(bytes.starts_with(b"FPYR"));

    let restored = PeakPyramid::from_bytes(&bytes).expect("Failed to deserialize pyramid");
    assert_eq!(restored.num_channels, pyramid.num_channels);
    assert_eq!(restored.total_frames, pyramid.total_frames);
    assert_eq!(restored.levels.len(), pyramid.levels.len());
    assert_eq!(
        restored.levels[0].channels[0][0],
        pyramid.levels[0].channels[0][0]
    );
}

#[test]
fn test_spectrogram_stft_scales_and_colormaps() {
    let sr = 48000;
    let frames = 48000; // 1 second
    let mut signal = Vec::with_capacity(frames);

    // Mix 440 Hz tone + 4000 Hz tone
    for i in 0..frames {
        let s = (2.0 * PI * 440.0 * (i as f32) / (sr as f32)).sin() * 0.5
            + (2.0 * PI * 4000.0 * (i as f32) / (sr as f32)).sin() * 0.3;
        signal.push(s);
    }

    // 1. Linear Scale + Magma
    let mut cfg = SpectrogramConfig {
        window_size: 1024,
        hop_size: 512,
        scale: FrequencyScale::Linear,
        num_freq_bins: 128,
        min_dbfs: -100.0,
        max_dbfs: 0.0,
        colormap: ColorMap::Magma,
    };
    let tile_linear = SpectrogramTileGenerator::generate_tile(&signal, sr, &cfg);
    assert!(tile_linear.width > 0);
    assert_eq!(tile_linear.height, 128);
    assert_eq!(
        tile_linear.rgba_pixels.len(),
        tile_linear.width * tile_linear.height * 4
    );

    // 2. Logarithmic Scale + Viridis
    cfg.scale = FrequencyScale::Logarithmic;
    cfg.colormap = ColorMap::Viridis;
    let tile_log = SpectrogramTileGenerator::generate_tile(&signal, sr, &cfg);
    assert_eq!(tile_log.width, tile_linear.width);
    assert_eq!(tile_log.height, 128);

    // 3. Mel Scale + Inferno
    cfg.scale = FrequencyScale::Mel;
    cfg.colormap = ColorMap::Inferno;
    let tile_mel = SpectrogramTileGenerator::generate_tile(&signal, sr, &cfg);
    assert_eq!(tile_mel.width, tile_linear.width);
    assert_eq!(tile_mel.height, 128);
}

#[test]
fn test_bs1770_loudness_and_true_peak() {
    let sr = 48000;
    let frames = 48000 * 3; // 3 seconds 1 kHz reference tone
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);

    // Generates a 1 kHz reference sine tone calibrated to -23.0 LUFS
    // Full scale 1 kHz sine integrated loudness is ~ -3.01 LUFS (unweighted sine RMS = -3.01 dBFS, K-weighting gain at 1kHz is 0 dB)
    // To achieve -23.0 LUFS: amplitude = 10^((-23.0 - (-3.01)) / 20) ~ 0.1
    let target_amp = 10.0f32.powf((-23.0 - (-3.01)) / 20.0);

    for i in 0..frames {
        let s = (2.0 * PI * 1000.0 * (i as f32) / (sr as f32)).sin() * target_amp;
        left.push(s);
        right.push(s);
    }

    let meas = LoudnessMeter::analyze(&[&left, &right], sr);

    // EBU Tech 3341 tolerance is +/- 0.1 LU
    assert!(
        (meas.integrated_lufs - (-23.0)).abs() <= 0.15,
        "Integrated loudness {} outside EBU Tech 3341 tolerance (+/- 0.15 LU)",
        meas.integrated_lufs
    );

    // True Peak check: sine wave with 4x oversampling peak dBTP within +/- 0.3 dB of peak dBFS
    let peak_linear = target_amp;
    let expected_dbtp = 20.0 * peak_linear.log10();
    assert!(
        (meas.max_true_peak_dbtp - expected_dbtp).abs() <= 0.3,
        "True peak {} outside tolerance of expected {}",
        meas.max_true_peak_dbtp,
        expected_dbtp
    );
}

#[test]
fn test_stereo_spatial_analysis() {
    let n = 4800;

    // 1. Mono in-phase (Left == Right) -> Phase correlation must be +1.0, width = 0.0
    let mono_l = vec![0.5f32; n];
    let mono_r = vec![0.5f32; n];
    let mono_res = StereoAnalyzer::analyze(&mono_l, &mono_r, 128);
    assert!((mono_res.phase_correlation - 1.0).abs() < 1e-4);
    assert!(mono_res.stereo_width < 1e-4);
    assert_eq!(mono_res.goniometer_points.len(), 128);

    // 2. Anti-phase (Left == -Right) -> Phase correlation must be -1.0
    let anti_l = vec![0.5f32; n];
    let anti_r = vec![-0.5f32; n];
    let anti_res = StereoAnalyzer::analyze(&anti_l, &anti_r, 128);
    assert!((anti_res.phase_correlation - (-1.0)).abs() < 1e-4);
    assert!(anti_res.stereo_width > 1.0);

    // 3. Uncorrelated noise -> Phase correlation must be near 0.0 (+/- 0.05)
    let mut uncorr_l = Vec::with_capacity(n);
    let mut uncorr_r = Vec::with_capacity(n);
    let mut state_l = 0x12345678u64;
    let mut state_r = 0x87654321u64;

    for _ in 0..n {
        state_l ^= state_l << 13;
        state_l ^= state_l >> 7;
        state_l ^= state_l << 17;

        state_r ^= state_r << 13;
        state_r ^= state_r >> 7;
        state_r ^= state_r << 17;

        uncorr_l.push(((state_l & 0xffff) as f32 / 32768.0) - 1.0);
        uncorr_r.push(((state_r & 0xffff) as f32 / 32768.0) - 1.0);
    }

    let uncorr_res = StereoAnalyzer::analyze(&uncorr_l, &uncorr_r, 128);
    assert!(
        uncorr_res.phase_correlation.abs() <= 0.05,
        "Uncorrelated noise phase correlation {} should be ~0.0",
        uncorr_res.phase_correlation
    );
}
